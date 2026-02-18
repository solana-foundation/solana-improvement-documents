---
simd: '0389'
title: Reduce account creation constant
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

Reduce the minimum deposit rate by 10x (set baseline
`min_deposit = 0.1 * min_balance_legacy`) and make this the new floor.

Introduce a supervisory integral controller targeting 5.5 GiB/epoch state growth
as the safety threshold. Under normal conditions, `min_deposit` remains pinned
at the 0.1x floor.

Depends on SIMD-0392 or another protocol change to allow for non-disruptive
minimum deposit increases.

## Motivation

Introduce a real-time price signal for state demand and apply back pressure to
prevent runaway state growth. The proposal gives the protocol more tuning knobs
(controller parameters such as deadband H, bounds, step sizes, clamps, target growth)
than a single fixed constant, enabling better behavior targeting. The target
state growth (5.5 GiB/epoch) is set from first principles based on validator
resource capacity (SSD size and upgrade cycle); see *Rationale for target state
growth* below.

## New Terminology

- `min_deposit`: The protocol-defined minimum lamports per byte required to
  allocate account data. Baseline is 0.1x the legacy minimum-balance constant
  (the floor). A supervisory integral controller MAY adjust it above the floor
  only when safety thresholds are exceeded.

## Detailed Design

### High level

- Upon activation, set `min_deposit = 0.1 * min_balance_legacy` (a 10x
  reduction) and treat this as the floor.
- Replace the current fixed minimum balance constant (legacy
  `min_balance_legacy`) used in account creation with a runtime-maintained
  `min_deposit` per-byte rate.
- A supervisory integral controller targets 5.5 GiB/epoch state growth as the safety
  threshold. The controller remains inactive unless the threshold is breached, in
  which case it engages through discrete adjustments of the `min_deposit` value.
- The absolute `min_deposit` value is clamped to remain between 0.1x and 1.0x
  of the legacy `min_balance_legacy` constant.
- When an account is allocated (either newly created or data size changed), the
  funder deposits `min_deposit * new_bytes` into the account.
- Post-execution min_balance checks must be relaxed so that minimum-deposit
  increases do not affect existing accounts. SIMD-0392 proposes such a change.

### Supervisory integral controller

The controller updates `min_deposit` every slot based on measured state growth
but is intended to be inactive during normal conditions (holding at the 0.1x
floor) and only engage when sustained growth exceeds a safety threshold.

**State tracking:**

- The integral accumulator `I` MUST be tracked in fork-aware bank state.
- The state growth for the current slot MUST also be tracked in fork-aware bank
  state via two per-slot deltas:
  - `data_size_delta` (bytes): accumulate only changes to `account.data.len`.
    The runtime already tracks per-instruction resize deltas; these MUST be
    accumulated into a single per-slot value. Additionally, for any accounts that
    end post-execution with zero lamports, their final data length MUST be
    subtracted from this delta (these deletions are not covered by the resize
    delta). Newly created accounts are covered by the resize delta.
  - `num_accounts_delta` (count): the net change in the number of accounts. Zero
    balance accounts are considered deleted.

The quantity thus tracked is the growth of on-chain account data (and per-account
overhead). On a validator, the accounts database and its indexes scale with this
state; so the accumulator effectively tracks the growth of the resource that
matters for capacity planning—accounts DB size plus index size on the accounts
disk.

**`G_slot` calculation:**

`G_slot` is the sum of the account data size growth plus the storage overhead
associated with changes in the number of accounts:

```
G_slot = data_size_delta + ACCOUNT_STORAGE_OVERHEAD * num_accounts_delta
```

where `ACCOUNT_STORAGE_OVERHEAD` is an existing in-consensus parameter used by
rent (128 bytes), representing the per-account storage overhead.

**Update rule (executed once per slot):**

Let target growth per slot be `G_target = 5.5 GiB/epoch / slots_per_epoch`.

Compute error: `e = G_slot - G_target` (positive if growth exceeds target).

Integrate with asymmetric bounds:

```
I_next = clamp( I + e, I_min = -4_000_000_000, I_max = +2_000_000_000 )
```

Deadband and discrete asymmetric steps:

```
if I_next >= H:                    # above-band (sustained over-target)
    min_deposit_next = min_deposit_current * 1.10   # +10%
elif I_next <= -H:                 # below-band (sustained under-target)
    min_deposit_next = min_deposit_current * 0.95   # -5%
else:
    min_deposit_next = min_deposit_current          # hold
```

Notes:

- No proportional term; the controller is integral-only with a deadband. The
  goal is to keep price pinned at the floor (0.1x) under normal conditions and
  react only to pronounced spikes.
- `H` (deadband threshold) is a protocol constant (see Protocol constants).
- `I` is the integral accumulator over error; large positive values indicate
  sustained overshoot; large negative values indicate sustained slack.

**Rationale for target state growth (G_target):**

Assume **4 TB** SSD capacity after a 2-year window; constants can be revisited
when typical validator capacity changes. The accumulator tracks growth of
accounts data + index (the storage that scales with state on the accounts disk).

Capacity budget: compressed snapshots are ~20% of total accounts data size with
2 full snapshots retained; ledger up to **500 GB**; current data + index **~500 GB**.
If data + index growth over 2 years is **2 TB**, total accounts size is 2.5 TB and
snapshot footprint is 2 × 0.2 × 2.5 TB ≈ **1 TB**. Then 4 TB − 1 TB (snapshots) −
0.5 TB (current data+index) − 0.5 TB (ledger) = **2 TB** for that growth. With
~182 epochs/year, 2 TiB over 364 epochs → **G_target = 5.5 GiB/epoch**.

Rationale for the controller design: the main goal of the controller is to
supervise state growth and engage effectively and predictably when the safety
threshold is violated.

- Asymmetric updates: bias towards upwards adjustments after engagement is
  triggered allows a faster response to excessive state growth and a more
  gradual return to baseline when conditions normalize.
- Asymmetric integral bounds: the absolute value of `I_min` being larger than
  `I_max` allows for the accumulation of a buffer to prevent smaller spikes
  from causing a `min_deposit` adjustment, thereby increasing stability of the
  0.1x floor price. A return to baseline can happen faster after a high
  allocation event has passed, with the lower `I_max` value.
- Deadband: avoids oscillations around the safety threshold.

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

The Rent sysvar account is `SysvarRent111111111111111111111111111111111`.

### Persistence and snapshot integrity

- The two per-slot deltas (`data_size_delta`, `num_accounts_delta`) and the
  integral accumulator `I` MUST be backed up into dedicated sysvars so they are
  covered by snapshot integrity checks via the accounts lthash. This avoids the
  need to include these values in the bank hash.
- On startup, validators MUST load these values exclusively from the sysvars,
  rather than any non-account serialized fields in the snapshot, ensuring that
  integrity-checked values are used.
- Sysvar accounts:
  - `SysvarAccountsDataSizeDe1ta1111111111111111`:
    carries the per-slot `data_size_delta` in bytes.
  - `SysvarAccountsNumDe1ta111111111111111111111`:
    carries the per-slot `num_accounts_delta` (net account count change).
  - `SysvarRentContro11er1ntegra1111111111111111`:
    carries the integral accumulator `I`.
  - `SysvarRent111111111111111111111111111111111` (existing):
    carries Rent fields and `min_deposit`.
- Update timing:
  - When the slot is advanced and a new bank is created, the accumulator `I`
    and the Rent sysvar (carrying `min_deposit`) MUST be updated then written
    to their respective sysvars.
  - At slot completion, the two per-slot deltas MUST be written to their
    respective per-slot sysvars. This ensures
    the integral and rent for the next slot have access to the correct
    delta values for the current (just-completed) slot, even in the case of
    restarting and loading from a snapshot.

### Protocol constants and feature gates

- New constants:
  - `G_target = 5.5 GiB/epoch` (see *Rationale for target state growth* above)
  - `H` deadband threshold for integral accumulator
  - `I_min = -4_000_000_000`, `I_max = +2_000_000_000`
  - `down_step = -5%`, `up_step = +10%`
  - `min_deposit_lower_bound = 0.1 * min_balance_legacy`
  - `min_deposit_upper_bound = 1.0 * min_balance_legacy`

## Alternatives Considered

- Keep a fixed global constant: simple but provides no feedback control and
  limited ability to respond to demand shocks.
- Proportional-only controller: reacts quickly but adjustments are more erratic;
  integral term addresses bias and long-term trends.
- Hard step adjustments per epoch: simpler but introduces large
  discontinuities and gaming incentives.
- Dynamic rent w/ eviction: more complete but controversial and
  complicated. This proposal will allow a reduction in state allocation cost
  sooner.

## Impact

- Dapp developers: Creation and reallocation costs vary over time; programs can
  query the Rent sysvar for current rates. Significant minimum-deposit
  reduction is expected.
- Validators: Minor overhead to track controller state (integral accumulator
  `I`, state growth measurements) and measure state growth per slot;
  predictable bounds via clamps. Fork-aware state management required.
- Core contributors: Runtime changes to account allocation flows, modified
  validity check logic with reallocation tracking; Rent sysvar exposure.

## Security Considerations

- Controller stability: Controller parameters (H, bounds, step sizes) MUST be
  selected to avoid oscillation; clamps provide guardrails. Per-slot updates
  provide fine-grained reactivity.
- Manipulation risk: integral term prevents sustained deviation.
- Determinism: Controller updates MUST be deterministic across validators given
  identical inputs. Fork-aware state tracking ensures consistency.

## Backwards Compatibility *(Optional)*

- The shift from a fixed minimum-balance constant to dynamic `min_deposit` with
  checks is a breaking change to account validity semantics, gated behind
  feature activation.
- No account metadata changes required; backwards compatible with existing account
  structures and snapshots.

## Dependencies *(Optional)*

This proposal depends on the following previously accepted or pending proposals:

- [SIMD-0392]: Relaxation of post-execution min_balance check — enables
  non-disruptive increases without per-account metadata.

[SIMD-0392]: https://github.com/solana-foundation/solana-improvement-documents/pull/392

