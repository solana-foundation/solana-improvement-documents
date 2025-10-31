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

Depends on SIMD-0392 or another protocol change to allow for non-disruptive rent
increases.

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

## Detailed Design

### High level

- Replace the current fixed minimum balance constant (legacy `min_balance_legacy`)
  used in account creation with a dynamic `min_deposit` per-byte rate maintained
  by the runtime.
- The `min_deposit` is adjusted every slot by a PI controller that targets a
  state growth rate of 1 GB per epoch.
- The absolute `min_deposit` value is clamped to remain between 0.1x and 1.0x
  of the legacy `min_balance_legacy` constant.
- When an account is allocated (either newly created or data size changed),
  the funder deposits `min_deposit * new_bytes` into the account.
- Post-execution min_balance checks must be relaxed so that rent increases do not
  affect existing accounts. SIMD-0392 proposes such a change.

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
  components of the controller, respectively. These coefficients essentially
  represent the 'weight' assigned to each component.
- `I` is the integral accumulator, steering controller output based on
  longer-term trends. A large positive `I` indicates consistent overshooting of
  the target, while a large negative value indicates slack available for future
  state growth.
- The proportional component is intended for immediate response to above target
  growth.

**Clamps:**

Apply clamps after the update to ensure `min_deposit` stays within bounds:

```
min_deposit_next = clamp(
  0.1 * min_balance_legacy,
  1.0 * min_balance_legacy,
  min_deposit_next
)
```

where `min_balance_legacy` is the fixed minimum-balance constant used prior to
this feature activation.

### Sysvar exposure

Expose `min_deposit` via the existing Rent sysvar to avoid introducing a new
sysvar and to preserve compatibility with existing on-chain programs.

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
  query the Rent sysvar (via the unified sysvar API) for current rates.
  Significant rent reduction is expected.
- Validators: Minor overhead to track controller state (integral accumulator `I`,
  state growth measurements) and measure state growth per slot; predictable
  bounds via clamps. Fork-aware state management required.
- Core contributors: Runtime changes to account allocation flows, modified
  validity check logic with reallocation tracking; Rent sysvar exposure via the
  unified sysvar API.

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

- The shift from a fixed minimum-balance constant to dynamic `min_deposit` with
  checks is a breaking change to account validity semantics, gated behind feature
  activation.
- No account metadata changes required; backwards compatible with existing account
  structures and snapshots.

## Dependencies *(Optional)*

This proposal depends on the following previously accepted or pending proposals:

- [SIMD-0392]: Relaxation of post-execution min_balance check — enables
  non-disruptive increases without per-account metadata.

[SIMD-0392]: https://github.com/solana-foundation/solana-improvement-documents/pull/392

