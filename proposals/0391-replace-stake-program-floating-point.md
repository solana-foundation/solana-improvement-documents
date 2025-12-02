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

Replace all floating-point (`f64`) arithmetic within the Solana Stake Program's
warmup and cooldown logic with a fixed-point implementation using integer
arithmetic. The new logic expresses the warmup/cooldown rate in basis points
(bps) and performs all proportional stake calculations using `u128`
intermediates.

This change is a prerequisite to the Stake Program's migration to a `no_std`
Pinocchio-based implementation and ensures compatibility with upstream eBPF
toolchains, which do not support floating-point operations.

## Motivation

The Stake Program's use of `f64` presents two blockers to the upcoming roadmap:

1. **Upstream eBPF incompatibility:** Standard eBPF strictly forbids
   floating-point operations. While the solana fork (SBF) currently supports `f64`
   via a deterministic (and inefficient) `soft-float` compiler built-in, aligning
   with upstream standards requires removing all `f64` usage from the program.

2. **Pinocchio migration inconsistency:** There is appetite for
   converting the Stake Program to a highly efficient, `no_std` Pinocchio
   implementation (reducing CU usage by +90%). These efforts are undermined by the
   immense cost of soft-float operations. [Benchmarking
   shows](https://solana.com/docs/programs/limitations#limited-float-support:~:text=Recent%20results%20show,Divide%20%20%20%20%20%209%20%20%20219)
   a 22x performance penalty for a single multiplication of an `f32` versus a
   `u64`. Using an `f64` with an operation like division is even more complex.
   Further, doing the float migration independently allows p-stake to enforce
   semantic equivalence for its migration.

## Requirements

The new implementation must be a replacement that precisely models the intent of
the original logic. Any resulting differences in output should be minor and a
direct result of improved numerical precision.

## New Terminology

None

## Detailed Design

### Proposed Fixed-Point Implementation

This proposal replaces `f64` with fixed-point arithmetic in basis points and
reorders operations to preserve precision while using integer arithmetic only.

#### Rate representation (basis points)

Instead of storing the warmup/cooldown rate as an `f64`, it is represented in
basis points (bps). The default (`0.25`) and new warmup (`0.09`) are encoded as:

```rust
pub const BASIS_POINTS_PER_UNIT: u64 = 10_000;
pub const ORIGINAL_WARMUP_COOLDOWN_RATE_BPS: u64 = 2_500; // 25%
pub const TOWER_WARMUP_COOLDOWN_RATE_BPS: u64 = 900; // 9%
```

with a new helper that determines the active rate based on the epoch:

```rust
pub fn warmup_cooldown_rate_bps(
    epoch: Epoch,
    new_rate_activation_epoch: Option<Epoch>,
) -> u64
```

The legacy `f64` constants and function are preserved but marked deprecated:

```rust
// All marked as deprecated as of 2.0.1
pub const DEFAULT_WARMUP_COOLDOWN_RATE: f64 = 0.25;
pub const NEW_WARMUP_COOLDOWN_RATE: f64 = 0.09;
pub fn warmup_cooldown_rate(
    current_epoch: Epoch,
    new_rate_activation_epoch: Option<Epoch>,
) -> f64
```

#### Reordered proportional stake formula

The original float logic computed:

```text
(account_portion / cluster_portion) * (cluster_effective * rate)
```

This is algebraically equivalent to the fixed-point re-ordering:

```text
change =
    (account_portion * cluster_effective * rate_bps) /
    (cluster_portion * BASIS_POINTS_PER_UNIT)
```

All multiplications are performed first in `u128` to maximize precision and
delay truncation. If the intermediate product would overflow, the numerator
saturates to `u128::MAX` before division and the final result is clamped to the
account's stake (`account_portion`), so the overflow path remains rate-limited
(fail-safe rather than fail-open).

#### New methods

The Delegation/Stake implementation exposes the integer math helpers under
new `_v2` entrypoints:

```rust
// === Integer math used under-the-hood ===
impl Delegation {
    pub fn stake_activating_and_deactivating_v2<T: StakeHistoryGetEntry>(
        ...
    ) -> StakeActivationStatus
    fn stake_and_activating_v2<T: StakeHistoryGetEntry>(...) -> (u64, u64)
}

impl Stake {
    pub fn stake_v2<T: StakeHistoryGetEntry>(...) -> u64
}
```

The pre-existing float-based functions remain under their original names for
API compatibility but are marked deprecated in favor of the `_v2` versions:

```rust
impl Delegation {
    #[deprecated(since = "2.0.1", note = "Use stake_v2() instead")]
    pub fn stake<T: StakeHistoryGetEntry>(...) -> u64

    #[deprecated(
        since = "2.0.1",
        note = "Use stake_activating_and_deactivating_v2() instead",
    )]
    pub fn stake_activating_and_deactivating<
        T: StakeHistoryGetEntry,
    >(...) -> StakeActivationStatus
}

impl Stake {
    #[deprecated(since = "2.0.1", note = "Use stake_v2() instead")]
    pub fn stake<T: StakeHistoryGetEntry>(...) -> u64
}
```

#### Minimum Progress Clamp (`max(1)`)

To match legacy behavior, the fixed-point implementation preserves a minimum
per-epoch change of 1 lamport for non-zero stake. This preserves the "always
make forward progress" invariant for both warmup and cooldown, ensuring small
delegations do not get stuck in activating/deactivating states due to
truncation.

### State Compatibility

To maintain backwards compatibility with on-chain stake account data, the
`Delegation` struct is modified as follows:

```diff
pub struct Delegation {
    pub voter_pubkey: Pubkey,
    pub stake: u64,
    pub activation_epoch: Epoch,
    pub deactivation_epoch: Epoch,
-   pub warmup_cooldown_rate: f64,
+   pub _reserved: [u8; 8],
}
```

This preserves the exact memory size and layout of existing accounts. It is a
legacy field anyway, with the actual rate being determined dynamically in
functions.

## Alternatives Considered

Tested a number of other libraries [have been
tested](https://github.com/grod220/stake-ebpf-check) for upstream bpf
compatibility.

| Method        | Result  | Notes                              |
|---------------|---------|------------------------------------|
| bnum          | Success | Requires using `u32` limbs         |
| crypto-bigint | Failure | Composite return types not allowed |
| fixed-bigint  | Failure | Composite return types not allowed |
| uint          | Failure | `__multi3` is not supported        |

Note also that this SIMD recommends using `u128` arithmetic. Currently, this is
_not_ supported in upstream bpf (`__multi3` error is
raised). [Llvm-project PR#168442](https://github.com/llvm/llvm-project/pull/168442)
is currently up to get upstream bpf support for it, and VM maintainers feel
confident it will be merged and included in the next release. For that reason,
scaled math (without a library) is preferred.

## Impact

### Entities

- **Stake Program:** The on-chain program is updated to use the new
  integer-based calculation helpers from `solana-stake-interface`. It now
  routes through `stake.delegation.stake_activating_and_deactivating_v2()`.

- **Agave:** Update the workspace dependency on
  `solana-stake-interface` and adopt the integer entrypoints
  (`Stake::stake_v2()` and `Delegation::stake_activating_and_deactivating_v2()`).
  behind feature gate.

- **Firedancer:** Will need to update their stake calculations in
  lock-step with the above integer-math changes.

### Differential Fuzzing

To quantify the numerical differences between the fixed-point implementation and
the legacy `f64` path, we run an additional prop test that:

- samples random non-zero `account`, `cluster_portion`, and
  `cluster_effective` values across the full `u64` range,
- exercises both the legacy `f64` formula and the new integer
  implementation at the current 9% rate

For 100,000 samples at the 9% rate we observe:

| Metric           | Value                    | Notes                        |
|------------------|--------------------------|------------------------------|
| Avg. abs. diff.  | 0.505 lamports           | Mean abs(candidate − oracle) |
| Avg. diff (ULPs) | 0.218 ULPs               | Avg. ULP distance of `f64`   |
| p50/p90/p95/p99  | 0 / 1 / 1 / 6 lamports   | Percentiles of abs. diff.    |
| Worst-case diff. | 932 lamports (1.82 ULPs) | Float imprecision at high #s |

In short, there is high agreement and minimal deviation in outputs. Over 50% of
results were identical, and 95% of results differed by at most 1 lamport. In the
worst case, the difference was still only a difference of 1.82 ULPs, confirming it
is an expected artifact of f64 precision limitations, not a logic error.

#### Note on ULPs

A "Unit in the Last Place" (ULP) measures the gap between adjacent representable
`f64` values. We use this metric to compare our new integer implementation
against the legacy float implementation. Because a `f64` cannot represent every
integer precisely past 2^53, the float-based result can differ slightly from the
integer-based one, even when both are logically correct. Measuring this
difference in ULPs allows us to verify the discrepancy is due to expected
floating-point artifacts, not a bug.

### Performance

For a sample configuration (`account_portion = 1_000_000_000`, `cluster_portion
= 100_000_000_000`, `cluster_effective = 5_000_000_000_000`,
`new_rate_activation_epoch = 50`), the results show a minor increase in CU
consumption for the new logic:

- **Legacy (`f64`) Implementation:** 985 CUs
- **New (`u128`) Implementation:** 1046 CUs

The fixed-point implementation is **6.2% more expensive** for this benchmark.
This result is due to the type widening to `u128` and checked math. However,
this is acceptable given the vast majority of CU costs are due to serialization
(improved by [zero-copy
p-stake](https://github.com/solana-foundation/solana-improvement-documents/pull/401)).

## Security Considerations

1. **Unit tests:** Baseline of correctness by testing specific, known
   scenarios and edge cases.
2. **Differential Fuzzing (`proptest`):**
    - Maintains an oracle implementation that preserves the original
      `f64` logic, used only in tests.
    - Runs the new integer implementation against the oracle over
      thousands of randomly generated inputs spanning the full `u64` domain.
    - Uses a ULP-based tolerance (`4 × ULP`) to account for the
      accumulated rounding error inherent in the float-based path while
      ensuring the integer implementation never deviates more than expected
      from the float oracle.
3. **External Audit:** A comprehensive audit from an auditor with good
   skills in numerical audits to validate arithmetic equivalence or regressions.
