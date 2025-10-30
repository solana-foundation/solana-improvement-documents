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

Replace of all floating-point (`f64`) arithmetic within the Solana
Stake Program's warmup and cooldown logic with a deterministic
fixed-point implementation using integer arithmetic. This change is a
prerequisite to the Stake Program's migration to a `no_std`
Pinocchio-based implementation and ensures compatibility with upstream
eBPF toolchains, which do not support floating-point operations.

## Motivation

The Stake Program's use of `f64` presents two blockers to the upcoming roadmap:

1. **Upstream eBPF incompatibility:** Standard eBPF strictly forbids
   floating-point operations. While the solana fork (SBF) currently
   supports `f64` via a deterministic (and inefficient) `soft-float`
   compiler built-in, aligning with upstream standards requires
   removing all `f64` usage from the program.

2. **Pinocchio migration inconsistency:** There is appetite for
   converting the Stake Program to a highly efficient, `no_std`
   Pinocchio implementation (reducing CU usage by +90%). These efforts
   are undermined by the immense cost of soft-float operations.
   [Benchmarking shows](https://solana.com/docs/programs/limitations#limited-float-support:~:text=Recent%20results%20show,Divide%20%20%20%20%20%209%20%20%20219)
   a 22x performance penalty for a single multiplication of an `f32`
   versus a `u64`. Using an `f64` with an operation
   like division is even more complex. Further, doing the float
   migration independently allows p-stake to enforce semantic
   equivalence for its migration.

## Requirements

The new implementation must be a replacement that precisely models the
intent of the original logic. Any resulting differences in output
should be minor and a direct result of improved numerical precision.

## New Terminology

None

## Detailed Design

### Proposed Fixed-Point Implementation

This proposal replaces `f64` with _rational arithmetic_, expressing
the `warmup_cooldown_rate` as a fraction and reordering operations to
preserve precision while using integer arithmetic.

- The floating-point rates will be converted to their fractional
  equivalents:
  - `DEFAULT_WARMUP_COOLDOWN_RATE` (`0.25`) becomes a fraction of
    **(numerator: 1, denominator: 4)**.
  - `NEW_WARMUP_COOLDOWN_RATE` (`0.09`) becomes a fraction of
    **(numerator: 9, denominator: 100)**.

- The current flow
  `(account_portion / cluster_portion) * (cluster_effective * rate)` is
  reordered to
  `(account_portion * cluster_effective * rate_numerator) / (cluster_portion * rate_denominator)`.
  Instead of performing divisions early in the process (which truncates
  intermediate results), all multiplications are performed first.

- All intermediate multiplications are performed using `u128` to
  prevent overflow.

### State Compatibility

To maintain backwards compatibility with on-chain stake account data,
the `Delegation` struct will be modified:

```diff
pub struct Delegation {
    pub voter_pubkey: Pubkey,
    pub stake: u64,
    pub activation_epoch: Epoch,
    pub deactivation_epoch: Epoch,
-   pub warmup_cooldown_rate: f64,
+    _reserved: [u8; 8],
}
```

This preserves the exact memory size and layout of existing accounts.
It is a legacy field anyway, with the actual rate being determined
dynamically in functions.

## Alternatives Considered

Decimal scaling factor. Uses a uniform scaling factor represent rates
as integers like BPS (e.g 0.25 = 2500). Rejected as rational
arithmetic is more precise. It may be easier to use for external
consumers, but these values are really only used internally to the
stake interface crate.

## Impact

- **Stake Program:** The program must be updated to use the new
  integer-based calculation helpers from `solana-stake-interface`. It is
  doing so mostly through its use of
  `stake.delegation.stake_activating_and_deactivating()`. Also, the new
  `Delegation` struct definition (with its private field), will likely
  impact how its being instantiated in a few areas.

- **Agave:** Update the workspace dependency on
  `solana-stake-interface`. Runtime stake processing already funnels
  through Delegation::stake_activating_and_deactivating, so a dependency
  bump automatically picks up the fixed-point math without touching
  Agave code.

- **Firedancer:** Will also need to update their stake interface
  dependency in lockstep with Agave.

## Security Considerations

1. **Unit tests:** Baseline of correctness by testing specific, known
   scenarios and edge cases.
2. **Differential Fuzzing (`proptest`):**
   - An oracle function preserving the original `f64` logic will be
     maintained for testing purposes only.
   - The test will run the new integer implementation against the oracle
     with millions of random inputs.
   - Assert that the results are within a relative tolerance to account
     for the increased precision.
3. **External Audit:** A comprehensive audit from an auditor with good
   skills in numerical audits to validate arithmetic equivalence or
   regressions.
