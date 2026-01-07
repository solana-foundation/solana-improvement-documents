---
simd: "0391"
title: Stake Program Float to Fixed-Point
authors:
  - Gabe (Anza)
  - Pete (Anza)
category: Standard
type: Core
status: Idea
created: 2025-10-23
feature: (to be assigned upon acceptance)
---

## Summary

This SIMD proposes replacing all IEEE-754 double-precision floating-point
arithmetic within the Solana Stake Program & validator client's warmup/
cooldown logic with a fixed-point implementation using integer arithmetic.
The new logic expresses the warmup/cooldown rate in basis points (bps) and
performs stake calculations using unsigned 128-bit integers to maintain
precision.

## Motivation

This change is a prerequisite to the Stake Program's migration to a `no_std`
& upstream eBPF-toolchain friendly implementation. Standard eBPF strictly
forbids floating-point operations. While the solana fork (SBF) allows for it
via a deterministic (and inefficient) `soft-float` compiler built-in,
aligning with upstream standards requires removing all floating-point usage
from the program.

The validator client shares the same warmup/cooldown calculation logic with
the on-chain program, so it is also in need of a lock-step update to stay in
sync.

## New Terminology

- **Basis points (bps)**: An integer representation of a percentage where
  `bps = percent × 100`.
    - 1 bps = 0.01%
    - 1% = 100 bps

- Formula variables
    - **account_portion**: The amount of stake (in lamports) for a single
      account that is eligible to warm up or cool down in a given epoch.
    - **cluster_portion**: The total amount of stake (in lamports) across the
      cluster that is in the same warmup/cooldown phase as `account_portion`
      for the previous epoch.
    - **cluster_effective**: The total effective stake in the cluster (in
      lamports) for the previous epoch.

## Detailed Design

### Rate representation (basis points)

The current network warmup/cooldown rate is 9%. This means that, in any given
epoch, at most 9% of the previous epoch's effective stake can be activated or
deactivated.

Currently, this figure is represented in floating-point: `0.09`. The new
representation is an integer of basis points: `900`.

### Maintaining precision

The original float logic computes:

```text
RATE_FLOAT = 0.09

allowed_change = (account_portion / cluster_portion) * (cluster_effective * RATE_FLOAT)
```

For an integer implementation, it's important to re-arrange the formula so
that the division happens last to maintain the highest precision. This is
achieved via an algebraically equivalent re-ordering:

```text
BASIS_POINTS_PER_UNIT = 10_000
RATE_BPS = 900

allowed_change =
    (account_portion * cluster_effective * RATE_BPS) /
    (cluster_portion * BASIS_POINTS_PER_UNIT)
```

Note: The division uses unsigned integer division so it truncates (rounds down).

#### Widening arithmetic to 128-bit integers

All inputs are unsigned 64-bit lamport quantities. Because of the extra
multiplication, all values used in the formula should be widened to unsigned
128-bit integers (or an exact emulation). The final value should then be cast
back down to an unsigned 64-bit integer.

Implementations that do not offer native unsigned 128-bit arithmetic must
emulate it (for example via fixed-width limb arithmetic).

#### Saturation and fail-safe behavior

If the intermediate multiplication overflows the maximum representable
unsigned 128-bit value, the numerator saturates to the maximum 128-bit value
before division. The result is then clamped to `account_portion`. This ensures
that overflow cannot amplify a stake change beyond the account's own portion
(fail-safe rather than fail-open).

### Minimum progress clamp

Currently, when `account_portion > 0`, there is a granted minimum change of 1
lamport per epoch so that small delegations do not get stuck in activating/
deactivating states due to truncation. The new implementation keeps this
behavior.

**Note:** This clamp applies only to stake activation/deactivation
transitions, not to inflation reward payouts. Reward distribution has a
separate mechanism that defers sub-lamport payouts by not advancing
`credits_observed` until a full lamport can be paid.

### Pseudocode guidance

#### Current implementation

```text
RATE_FLOAT = 0.09

# All params are unsigned 64-bit integers
function rate_limited_stake_change(account_portion, cluster_portion, cluster_effective):
    if account_portion == 0 or cluster_portion == 0 or cluster_effective == 0:
        return 0

    # Cast all params to double
    weight_float = account_portion_float / cluster_portion_float
    allowed_change_float = weight_float * cluster_effective_float * RATE_FLOAT

    # Truncate toward zero via cast
    allowed_change = allowed_change_float as unsigned 64-bit integer

    # Never allow more than the account's own portion to change
    if allowed_change > account_portion:
        allowed_change = account_portion

    # Minimum progress clamp
    if allowed_change == 0:
        return 1

    return allowed_change
```

#### Proposed new implementation

```text
BASIS_POINTS_PER_UNIT = 10_000
RATE_BPS = 900

# All params are unsigned 64-bit integers
function rate_limited_stake_change(account_portion, cluster_portion, cluster_effective):
    if account_portion == 0 or cluster_portion == 0 or cluster_effective == 0:
        return 0

    # Cast all params to unsigned 128-bit integer
    # All multiplications saturate
    numerator = account_portion_128 * cluster_effective_128 * RATE_BPS_128

    denominator = cluster_portion_128 * BASIS_POINTS_PER_UNIT_128

    allowed_change_128 = numerator / denominator

    # Never allow more than the account's own portion to change
    if allowed_change_128 > account_portion_128:
        allowed_change_128 = account_portion_128

    # Narrow back to unsigned 64-bit integer
    allowed_change = allowed_change_128 as unsigned 64-bit integer

    # Minimum progress clamp
    if allowed_change == 0:
        return 1

    return allowed_change
```

## Alternatives Considered

The primary alternative is to continue using floating-point arithmetic. For
reasons given in the motivation section, this blocks upstream eBPF-toolchain
usage, which just puts the technical debt off to handle later.

## Impact

- **Stake Interface**:
    - Export new integer-based stake activation and deactivation logic for rust
      consumers
    - Deprecate the floating-point rate field while preserving binary layout
      compatibility

- **Stake Program**: Feature gate v2 interface helpers in:
    - **Stake Merging**: Stake calculations are used to determine if the
      account is in a transient state, ensuring that merges are rejected if the
      account is not effectively fully active or inactive.
    - **Stake Splitting**: Stake calculations are used determine if the source
      stake is currently active (effective stake > 0). This status is required
      to correctly enforce rent-exempt reserve prefunding requirements for the
      destination account.
    - **Stake Redelegation**: The account's cooldown status is determined with
      stake calculations and confirms that effective stake is exactly zero
      before allowing redelegation.
    - **Stake Withdrawal**: When withdrawing from a deactivated account, stake
      calculations are used to determine the remaining effective stake.

- **Validator Clients (Agave & Firedancer)**: Clients must feature gate the
  transition from floating-point to fixed-point arithmetic in all
  consensus-critical operations involving effective, activating, or
  deactivating stake. The following operations require updates:
    - **Stake Activation and Deactivation**: When querying a stake delegation's
      status for a given epoch, the validator _computes how much of the
      delegation's stake has completed warmup or cooldown_. This requires
      walking through epochs from the delegation's activation or deactivation
      point, computing the allowed stake change at each epoch boundary to
      determine the portion that transitioned. The result categorizes the
      delegation's lamports into effective, activating, and deactivating
      buckets.
    - **Epoch Boundary Stake History**: At each epoch boundary, the validator
      iterates over all stake delegations and _computes their activation status_
      as of the concluding epoch. These per-delegation values are summed to
      produce the cluster-wide totals (effective/activating/deactivating) that
      form the new stake history entry. This entry is then used as input for
      subsequent epoch calculations.
    - **Stake Cache Updates**: The validator maintains a cache mapping vote
      accounts to their delegated stake. When a stake account is
      created/modified/closed, the cache entry for the associated vote account
      must be updated. This requires _computing the delegation's effective stake_
      contribution before and after the change to correctly adjust the cached
      totals.
    - **Vote Account Stake Totals**: At epoch boundaries, the validator
      refreshes the stake distribution across vote accounts for the upcoming
      epoch. For each vote account, it _sums the effective stake_ of all
      delegations pointing to that account. These totals determine leader
      schedule weights and fork choice voting power.
    - **Inflation Rewards**: Reward calculation iterates over each epoch in a
      vote account's credit history. For each epoch, the validator _computes the
      delegation's effective stake_ at that epoch, multiplies by the earned vote
      credits to produce points and accumulates these across epochs. The final
      reward is proportional to the delegation's share of total cluster points.
        - Note: Only the effective stake computation (warmup/cooldown) is
          affected by this SIMD. The downstream reward-to-lamport conversion
          and sub-lamport deferral logic remain unchanged.

## Security Considerations

All implementations must adhere to the following standards:

1. **Unit tests:** Baseline of correctness by testing specific, known
   scenarios and edge cases.
2. **Differential Fuzzing:** maintains an oracle implementation that preserves
   the original logic, used only in tests. Those should then be run against
   the integer arithmetic to ensure a difference of no more than `4 x ULP`
   (units of last place).
3. **External Audit:** A comprehensive audit from an auditor with good skills
   in numerical audits to validate arithmetic equivalence or regressions.
