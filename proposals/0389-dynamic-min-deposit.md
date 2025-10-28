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
A 10% burn on allocations discourages churn. Requires account versioning to
track cumulative deposits in account metadata.

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
  checks. Set to `0.9 * min_deposit * data_size`, allowing accounts to pass
  balance checks when only 90% of the deposit is transferred to the account
  (with 10% burned at allocation time).

## Detailed Design

### High level

- Replace the current fixed minimum balance constant (legacy `min_balance_legacy`)
  used in account creation with a dynamic `min_deposit` per-byte rate maintained
  by the runtime.
- The `min_deposit` is adjusted every slot by a PI controller that targets a
  state growth rate of 1 GB per epoch.
- The absolute `min_deposit` value is clamped to remain between 0.1x and 1.0x
  of the legacy `min_balance_legacy` constant.
- For validity checks, `min_balance = 0.9 * min_deposit * data_size`. This
  allows the 10% burn to occur at allocation time while still passing balance
  checks.
- When an account is allocated (either newly created or data size increased),
  the funder pays `min_deposit * new_bytes`. At allocation time, 10% is burned
  (removed from cluster capitalization) and 90% is deposited to the account.
- Ephemeral accounts (opened and closed within the same transaction) still incur
  the 10% burn cost.
- Cumulative `min_deposit` paid over an account's lifetime is recorded in the
  account's metadata and included in the accounts lattice hash (LTHash) input.
  Any metadata change implies an LTHash update.

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

### Allocation and deallocation

**Upward reallocation (growth):**

For any instruction that allocates account data (including system create,
allocate, reallocate upward):

1. **Compute deposit:** The runtime computes the required deposit for the
   **incremental bytes only** using the current `min_deposit`:
   
   ```
   incremental_deposit = min_deposit * new_bytes
   burn_amount = incremental_deposit * 0.10
   net_deposit = incremental_deposit - burn_amount
   ```

2. **Transfer and burn:** Transfer `net_deposit` lamports from funder to the
   account. Burn `burn_amount` (removed from cluster capitalization; never
   enters any account balance).
   
   Update metadata:
   
   ```
   account.min_deposit_lamports += incremental_deposit
   ```

3. **Validity checks (post-execution):** Check that account balance satisfies:
   
   ```
   account.lamports >= min_balance
   where min_balance = 0.9 * account.min_deposit_lamports
   ```
   
   This check passes because `net_deposit = 0.9 * incremental_deposit` was
   transferred in step 2.

**Ephemeral account cost:**

Accounts that are allocated and deallocated within the same transaction still
incur the 10% burn cost at allocation time. This is necessary because tracking
whether an account was created in the current transaction to conditionally apply
the burn adds significant implementation complexity. The burn acts as a fee for
state growth, even if temporary.

Ephemeral accounts are important for many use-cases so it may be worthwhile to
provide a separate burn-free allocation path for them.

**Downward reallocation (shrinking):**

- When an account's data size is reduced, the tracked `min_deposit_lamports` is
  reduced proportionally to the size reduction:
  
  ```
  bytes_removed = old_size - new_size
  reduction = account.min_deposit_lamports * (bytes_removed / old_size)
  account.min_deposit_lamports -= reduction
  ```
  
- No lamports are burned on downward reallocation.
- No lamports are refunded to the fee payer (previously burned amounts remain
  burned).

### Account metadata and hashing

- Extend account metadata to include `min_deposit_lamports`, the cumulative
  `min_deposit` paid over all allocations for the account's data. This value
  increases when data size grows and decreases proportionally when data size
  shrinks.
- This extension requires **account versioning** to safely add new fields to the
  account structure without breaking existing snapshots or tooling.
- For accounts created before feature activation, `min_deposit_lamports` is
  initialized to `min_balance_legacy * account_data_size` on first access after
  activation (e.g., during snapshot load or first reallocation).
- Any change to this field updates the account's lattice hash input; therefore,
  LTHash calculations MUST be updated to incorporate the new field
  deterministically.
- This metadata is runtime-only and not exposed to on-chain program ABIs beyond
  its impact on hashing and rent/creation rules.

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
  - `burn_ratio = 10%`
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
- Core contributors: Runtime changes to account allocation flows, allocation-time
  burn accounting, account versioning for metadata schema extension, LTHash
  inclusion, new sysvar.

## Security Considerations

- Controller stability: Gains MUST be selected to avoid oscillation; clamps
  provide guardrails. Per-slot updates provide fine-grained reactivity.
- Manipulation risk: Attempts to game measured growth by bursty allocations are
  countered by the burn and proportional response; integral term prevents
  sustained deviation.
- Determinism: Controller updates MUST be deterministic across validators given
  identical inputs. Fork-aware state tracking ensures consistency.
- Burn timing: The burn occurs at allocation time (before validity checks),
  ensuring the account receives exactly 90% of the computed deposit. Validity
  checks use `min_balance = 0.9 * min_deposit_lamports` to match this.
- Ephemeral account impact: Accounts created and closed within the same
  transaction incur the 10% burn cost. This is an intentional tradeoff to avoid
  implementation complexity of tracking intra-transaction account lifecycles.
  It may be worthwhile to provide an alternative creation path specifically for
  ephemeral accounts.

## Backwards Compatibility *(Optional)*

- Requires account versioning to extend the account metadata structure with
  `min_deposit_lamports` field.
- New sysvar and syscall are additions; existing programs unaffected.
- The shift from fixed `min_balance` to dynamic `min_deposit` with 0.9x factor
  for balance checks is a breaking change to account validity semantics, gated
  behind feature activation.

