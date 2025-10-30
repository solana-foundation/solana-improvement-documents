---
simd: '0389'
title: Dynamic minimum deposit for account creation
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-10-27
feature: (fill in with feature key and github tracking issues once accepted)
supersedes:
superseded-by:
extends:
---

## Summary

Replace the fixed minimum balance constant with a dynamic `min_deposit` per-byte
rate adjusted every slot by a PI controller targeting 1 GB/epoch state growth.
Balance checks enforce that accounts maintain the rent price from their creation
time unless reallocated.

## Motivation

Introduce a real-time price signal for state demand and apply back pressure to
prevent runaway state growth. The proposal gives the protocol more tuning knobs
(PI gains, clamps, target growth) than a single fixed constant, enabling
better behavior targeting. With an
average on-chain state growth currently around ~200 MB per epoch, targeting 1
GB per epoch allows significant rent reductions while keeping growth bounded and
predictable.

## New Terminology

- `min_deposit`: The protocol-defined minimum lamports per byte required to
  allocate account data. Determined dynamically by the PI controller and updated
  every slot. Used to compute the deposit required for new allocations.
- `min_balance`: The minimum account balance required to pass runtime validity
  checks. Derived from the lesser of the account's current balance (preserving
  the rent price at creation) or the current `min_deposit * data_size` (adopting
  the new price after reallocation).

## Detailed Design

### High level

- Replace the current fixed minimum balance constant (legacy `min_balance_legacy`)
  used in account creation with a dynamic `min_deposit` per-byte rate maintained
  by the runtime.
- The `min_deposit` is adjusted every slot by a PI controller that targets a
  state growth rate of 1 GB per epoch.
- The absolute `min_deposit` value is clamped to remain between 0.1x and 1.0x
  of the legacy `min_balance_legacy` constant.
- When an account is allocated (either newly created or data size increased),
  the funder pays `min_deposit * new_bytes` which is deposited to the account.
- Balance checks ensure accounts never dip below the rent price paid at their
  creation time, unless reallocated (which adopts the current rent price).

### PI controller

The controller updates `min_deposit` every slot based on measured state growth.

**State tracking:**

- The integral accumulator `I` MUST be tracked in fork-aware bank state.
- Each slot measures realized state growth `G_slot` (bytes of new account data
  allocated minus freed/deallocated data).

**Update rule (executed once per slot):**

Let target growth per slot be `G_target = 1 GiB/epoch / slots_per_epoch`.

Compute error: `e = G_slot - G_target` (positive if growth exceeds target).

Update `min_deposit`:

```
min_deposit_next = min_deposit_current * exp( Kp * e / G_target + Ki * I )
I_next = I + e / G_target
```

The exponential form ensures `min_deposit` remains positive and provides
multiplicative (percentage-based) adjustments rather than additive changes.

- Gains `Kp`, `Ki` are protocol constants used by the proportional and integral
  components of the controller, respectively.
- `I` is the integral accumulator, steering controller output based on
  longer-term trends. A large positive `I` indicates consistent overshooting of
  the target, while a large negative value indicates slack available for future
  state growth.

**Clamps:**

Apply clamps after the update to ensure `min_deposit` stays within bounds:

```
min_deposit_next = clamp(
  0.1 * min_balance_legacy,
  1.0 * min_balance_legacy,
  min_deposit_next
)
```

where `min_balance_legacy` is the fixed minimum balance constant used prior to
this feature activation.

### Allocation and balance checks

**Account creation and upward reallocation:**

For any instruction that allocates account data (including system create,
allocate, reallocate upward):

1. **Compute deposit:** The runtime computes the required deposit for the
   **incremental bytes only** using the current `min_deposit`:
   
   ```
   incremental_deposit = min_deposit * new_bytes
   ```

2. **Transfer:** Transfer `incremental_deposit` lamports from funder to the
   account.
   
3. **Mark reallocation:** Flag the account as having been reallocated in this
   transaction (for post-execution checks).

**Downward reallocation:**

- No lamports are refunded to the fee payer when shrinking.
- Mark the account as having been reallocated in this transaction.

**Balance checks (pre-execution and post-execution):**

Before and after each transaction, validate account balance:

```
min_balance = if account.was_reallocated_in_tx {
    // Adopt current rent price
    min_deposit * account.data_size
} else {
    // Preserve rent price from creation; never force adoption of new price
    min(min_deposit * account.data_size, account.lamports)
}

assert(account.lamports >= min_balance)
```

**Rationale:**

The inductive proof: If an account is created with balance satisfying the
current rent price (enforced at creation), it can never dip below that price
unless explicitly reallocated. Reallocation adopts the new current rent price.
This avoids tracking per-account metadata while still enforcing rent-like
invariants.


### Pre-activation account handling

Accounts created before feature activation are treated as if they have always
satisfied the rent price at creation. The `min` operator in the balance check
ensures these accounts are never forced to adopt a higher rent price unless
explicitly reallocated.

### Sysvar exposure

A new sysvar `MinDeposit` MUST be added to expose the current `min_deposit`
value to on-chain programs.

**Sysvar ID:** `SysvarMinDeposit111111111111111111111111111`

```rust
pub struct MinDeposit {
    pub lamports_per_byte: u64,
}
```

- The sysvar is updated every slot after the controller computes `min_deposit_next`.
- Programs can read this sysvar to query the current deposit rate for allocation
  cost estimation.
- Accessible via the unified `sol_get_sysvar` syscall (SIMD-0127) using the
  sysvar ID above.

### Protocol constants and feature gates

- New constants:
  - `G_target = 1 GiB/epoch`
  - `Kp`, `Ki` controller gains
  - `min_deposit_lower_bound = 0.1 * min_balance_legacy`
  - `min_deposit_upper_bound = 1.0 * min_balance_legacy`

## Alternatives Considered

- Keep a fixed global constant: simple but provides no feedback control and
  limited ability to respond to demand shocks.
- Proportional-only controller: reacts quickly but adjustments are more erratic;
  integral term addresses bias and long-term trends.
- Hard step adjustments per epoch: simpler but introduces large
  discontinuities and gaming incentives.
- Dynamic rent w/ eviction: more complete but controversial and complicated.
  This proposal will allow a reduction in rent sooner.

## Impact

- Dapp developers: Creation and reallocation costs vary over time; programs can
  query the `MinDeposit` sysvar for current rates. Tooling SHOULD surface
  current `min_deposit` estimates. Expected rent reductions due to lower average
  state growth target.
- Validators: Minor overhead to track controller state (integral accumulator `I`,
  state growth measurements) and measure state growth per slot; predictable
  bounds via clamps. Fork-aware state management required.
- Core contributors: Runtime changes to account allocation flows, modified
  balance check logic with reallocation tracking, new sysvar.

## Security Considerations

- Controller stability: Gains MUST be selected to avoid oscillation; clamps
  provide guardrails. Per-slot updates provide fine-grained reactivity.
- Manipulation risk: Attempts to game measured growth by bursty allocations are
  countered by the proportional response; integral term prevents sustained
  deviation.
- Determinism: Controller updates MUST be deterministic across validators given
  identical inputs. Fork-aware state tracking ensures consistency.
- Balance preservation: The balance check mechanism ensures accounts can never be
  forced to adopt a higher rent price unless explicitly reallocated, protecting
  existing accounts from retroactive rent increases.

## Backwards Compatibility *(Optional)*

- New sysvar is an addition; existing programs unaffected.
- The shift from fixed `min_balance` to dynamic `min_deposit` with balance-based
  checks is a breaking change to account validity semantics, gated behind feature
  activation.
- No account metadata changes required; backwards compatible with existing account
  structures and snapshots.

