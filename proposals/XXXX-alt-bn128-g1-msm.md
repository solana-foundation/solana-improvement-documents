---
simd: 'XXXX'
title: BN254 G1 multi-scalar multiplication syscall (`alt_bn128_g1_msm`)
authors:
  - nzengi (independent)
category: Standard
type: Core
status: Review
created: 2026-05-10
feature:
supersedes:
superseded-by:
extends:
---

## Summary

Add a single Solana BPF syscall that computes Σᵢ scalarsᵢ · pointsᵢ over BN254 G1 using a Pippenger-style multi-scalar multiplication (MSM). The new syscall replaces the pattern of calling `alt_bn128_g1_multiplication_be` n times followed by `alt_bn128_g1_addition_be` (n − 1) times, which is the only on-chain MSM strategy available today.

This work is paired with [`halo2-solana-verifier`](https://github.com/nzengi/Solana-Plonk), the first PSE-Halo2 BN254/KZG/SHPLONK verifier deployed to Solana. That verifier's per-tx CU profile motivates the proposal: 62% of its 2,710,424 CU runs through SHPLONK's sequential G1 MSM. A batched MSM syscall is the highest-leverage cure, with directly measurable savings.

## Motivation

The existing G1 syscalls — `alt_bn128_g1_addition_be` (334 CU) and `alt_bn128_g1_multiplication_be` (3,840 CU), defined in [SIMD-0284](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0284-alt-bn128-little-endian.md) — are sufficient for Groth16-style verifiers (1 pairing + 2-3 G1 ops). They are *not* sufficient for verifier protocols whose dominant cost is **multi-scalar multiplication of many points**. Halo2 KZG/SHPLONK, Plonk variants, and any folding scheme with batched openings hit this regime.

### Concrete case study: Halo2 verifier on Solana

The Halo2 verifier above runs end-to-end on Solana BPF in 2,710,424 CU under Mollusk with `request_heap_frame(256_000)` (full per-stage breakdown: [`docs/cu_profile.md`](https://github.com/nzengi/Solana-Plonk/blob/main/docs/cu_profile.md)). Its per-stage breakdown is:

| Stage                       |       CU |    %  |
|-----------------------------|---------:|------:|
| read_proof                  |  258,633 | 10%   |
| lagrange::evaluate_lagrange |  533,709 | 20%   |
| compute_expected_h_eval     |  156,502 |  6%   |
| **shplonk::verify_opening** | **1,667,016** | **62%** |
| pairing                     |   49,546 |  2%   |
| **TOTAL**                   | **2,710,424** | 100% |

Mainnet's per-tx CU cap is 1,400,000. The verifier therefore aborts mid-SHPLONK on devnet today: tx [`3r1ZSg3D…XUje5`](https://explorer.solana.com/tx/3r1ZSg3DX6JhWp3zupEqqUptyz8GGpFekoqkjfyepBZySDCScMo5DAZYtwHpAM6cFw2Zajfchw7K7hho6YGXUje5?cluster=devnet) consumed 1,399,644 of 1,399,700 CU before hitting the meter.

`shplonk::verify_opening`'s cost is dominated by ~25 sequential `alt_bn128_g1_multiplication_be` calls inside its rotation-set inner loop. The proposed syscall collapses those into one batched call.

### Why this can't be solved in pure BPF

A pure-BPF Pippenger MSM was implemented and benchmarked under Mollusk in [`crates/g1-msm-ref`](../../crates/g1-msm-ref/) and [`programs/g1-msm-bench`](../../programs/g1-msm-bench/). Results at `heap_size = 256KB`:

| n  | sequential syscall | pippenger BPF      | naive BPF       |
|---:|-------------------:|-------------------:|----------------:|
|  2 |              8,558 |         18,188,362 |      12,743,035 |
|  4 |             17,356 |         33,884,582 |      24,988,700 |
|  8 |             34,964 |         58,145,235 |      49,953,590 |
| 16 |             70,180 | **heap exhausted** |     100,083,112 |
| 32 |            140,616 | **heap exhausted** |     199,589,804 |
| 64 |            281,494 | **heap exhausted** |     403,144,846 |

Pippenger inside BPF runs out of heap at n = 16 (each window allocates a `Vec<G1Projective>`-sized bucket array; many windows ⇒ heap thrash even at 256 KB). Naive scalar-mul-and-add in pure BPF reaches mainnet's CU cap immediately — that's why the existing `alt_bn128_g1_multiplication_be` syscall exists in the first place.

The conclusion: **Pippenger MSM has to land natively, as a syscall.** Pure-BPF is not a path here.

## New Terminology

* **MSM (multi-scalar multiplication)**: the operation Σᵢ sᵢ · Pᵢ over an elliptic-curve group, given sequences `(s₁, …, sₙ)` of scalars and `(P₁, …, Pₙ)` of group elements.
* **Pippenger algorithm**: the standard MSM algorithm with cost roughly `O(n·b / log n)` for n points and b-bit scalars, beating sequential scalar-mul-and-add by a factor proportional to the optimal window size.
* **Window size c**: the chunk size of scalar bits processed per Pippenger pass. The reference impl uses `c = ⌊log₂ n⌋ + 2`, which closely tracks the empirical optimum.

## Detailed Design

### Opcode

Add to `solana-define-syscall`:

```rust
pub const ALT_BN128_G1_MSM_BE: u64 = 7;
```

(Numbering follows the alt_bn128 series defined in SIMD-0302: 4 = G2_ADD, 5 = G2_SUB, 6 = G2_MUL.)

### Wire format

```
ALT_BN128_G1_MSM_HEADER_LEN: u64 = 4
  (u32 LE — n, the number of (scalar, point) pairs)

ALT_BN128_G1_MSM_PER_PAIR_LEN: u64 = 96
  (32 B BE Fr scalar + 64 B BE G1Affine point)

ALT_BN128_G1_MSM_OUTPUT_LEN: u64 = 64
  (BE G1Affine, x ‖ y; identity = all zeros)
```

Input layout, with scalars and points **grouped** rather than interleaved:

```
[0..4]                : n (u32 LE)
[4..4+32n]            : scalars     — each 32 B BE Fr
[4+32n..4+96n]        : points      — each 64 B BE G1Affine (x ‖ y)
```

Total input size: `4 + 96n` bytes.

The grouped layout matches arkworks's `VariableBaseMSM::msm` directly (no internal reorder buffer). EIP-2537 took the interleaved approach for BLS12-381; we deliberately diverge to follow Solana's [SIMD-0284](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0284-alt-bn128-little-endian.md) byte-format ergonomics that programs already consume via `solana-bn254`.

### Endianness

Big-endian, consistent with `alt_bn128_g1_addition_be`, `alt_bn128_g1_multiplication_be`, and the `_be` suffix convention introduced in SIMD-0284. An `_le` variant follows trivially when needed.

### Validation

Each input point MUST pass:

1. **Field check** — both x and y in `[0, p)` where p is the BN254 base-field modulus.
2. **Curve equation check** — `y² ≡ x³ + 3 (mod p)`.

The **subgroup check is skipped**, matching the convention for the existing `alt_bn128_g1_*` syscalls (BN254 G1 has cofactor 1, so all on-curve points are in the prime-order subgroup automatically). This is consistent with SIMD-0302's note: *"For existing G1 operations… cofactor of 1… no separate subgroup check is needed."*

Each input scalar is reduced modulo the BN254 scalar-field order before use. Non-canonical scalar encodings (≥ p) are accepted via reduce-mod-p, matching `alt_bn128_g1_multiplication_be`'s current behaviour. (A strict-canonical variant could be added, but introducing both behaviours under one syscall complicates the cost model; we leave that to a follow-up if needed.)

### Edge cases

| Input | Output |
|---|---|
| n = 0 | 64-byte zero (identity) |
| All scalars zero | 64-byte zero |
| All points identity | 64-byte zero |
| Scalar = 0 OR point = identity at index i | term skipped, others sum normally |
| Scalar non-canonical (≥ p) | reduce mod p, proceed |
| Point off-curve | `AltBn128Error::InvalidInputData` |

### CU cost

The proposed cost model is linear in n with a small fixed base:

```
cost(n) = 4_000 + n × 2_400 CU
```

Derivation:

* **Base 4,000 CU**: matches the existing fixed component of `alt_bn128_pairing_be` (sha256 + 36k constant ÷ 9 ≈ 4k for setup-heavy syscalls). Includes argument deserialization, scalar normalisation, and Pippenger window setup.
* **Per-point 2,400 CU**: derived from Pippenger's amortised cost in native code on x86 (where each elliptic-curve add+double in arkworks runs in ~50 ns ≈ 100 cycles). The existing `alt_bn128_g1_multiplication_be` is 3,840 CU per call; Pippenger amortises that down to roughly 2,400 CU per point at n ≥ 4.

Comparison with sequential syscall calls (`n × 3,840 + (n − 1) × 334`):

| n  |   sequential CU    |    proposed MSM CU   |  saving |
|---:|-------------------:|---------------------:|--------:|
|  2 |              8,014 |                8,800 |    −10% |
|  4 |             16,362 |               13,600 |    +17% |
|  8 |             33,058 |               23,200 |    +30% |
| 16 |             66,450 |               42,400 |    +36% |
| 32 |            133,234 |               80,800 |    +39% |
| 64 |            266,802 |              157,600 |    +41% |

(Mollusk-measured `syscall-seq` numbers, repeated from above, confirm this shape: at n = 64 the sequential path costs 281,494 CU; the proposed SIMD costs 157,600 CU — a 1.79× ratio.)

The MSM syscall is **slightly more expensive than two sequential calls at n = 2**, and **strictly cheaper than sequential calls at n ≥ 4**. The crossover is intentional and benign: callers with n ≤ 3 should keep using the existing per-point syscalls.

## Alternatives Considered

### A. Pure-BPF Pippenger

Implement Pippenger inside the verifier program, no new syscall. Rejected: the reference impl in [`crates/g1-msm-ref`](../../crates/g1-msm-ref/) runs out of BPF heap at n = 16 even with `request_heap_frame(256KB)`. Without the SIMD this path is non-viable above small n. Naive BPF MSM is also non-viable: 100M+ CU at n = 16, far past the per-tx cap.

### B. Multiple `alt_bn128_g1_multiplication_be` calls (status quo)

Keep the sequential per-point syscall pattern. The current Halo2 verifier uses this and aborts at the 1.4M cap; see `cu_profile.md` for the on-chain evidence. This is a hard ceiling, not a soft one.

### C. EIP-2537-style interleaved format

Use `[scalar₀ | point₀ | scalar₁ | point₁ | …]` instead of grouped. Rejected: incurs an internal reorder copy in the syscall implementation (arkworks expects grouped); diverges from Solana's `solana-bn254` byte layout conventions that programs already produce via `convert_endianness`.

### D. Combined MSM + pairing accumulator

A single syscall that does MSM and feeds the result into a deferred pairing equation. More aggressive, but couples two unrelated operations and complicates the cost model. The clean two-syscall split (this SIMD + the existing pairing) is preferable: each is independently optimisable, and the existing pairing syscall already does its job well (49,546 CU for 2 pairs in our verifier).

## Impact

For the [`halo2-solana-verifier`](https://github.com/nzengi/solana-poc) PoC at k = 4 StandardPlonk:

| Metric                                | Today   | With SIMD-XXXX  | Δ           |
|---------------------------------------|---------|-----------------|-------------|
| Total verify CU (Mollusk)             | 2,710,424 | ~2,180,000   | −20%        |
| `shplonk::verify_opening` slice       | 1,667,016 | ~1,140,000   | −32%        |
| Fits in single tx (1.4M cap)?         | No      | Marginal — 2-tx still likely | needs Layer 3 |

The −32% saving on `shplonk::verify_opening` is composed of:
* The syscall-layer saving documented in the table above (~50k CU at n = 25).
* The downstream restructuring savings: the verifier's per-iteration Fr coefficient computation collapses when MSM input prep is done once instead of inside an inner loop. This part is implementation-dependent; a refactor branch is being prepared in the PoC repo to validate the figure.

Beyond Halo2, every BN254 ZK system on Solana that does batched openings benefits: aggregator-style Groth16 (where multiple proofs are combined), folding schemes (Nova-style), Plonk variants (`alt_bn128_g1_msm` is the natural primitive for "verifier-side commitment combination" used in batched verifiers).

## Security Considerations

### Cryptographic

The Pippenger algorithm produces the same output as naive scalar-mul-and-add for the same inputs (modulo well-known curve identity edge cases, handled in the edge-cases table). It introduces no new security assumptions.

The skipped subgroup check is consistent with existing G1 syscalls and is sound for BN254 (cofactor 1).

### Implementation

The native implementation must be **constant-time with respect to scalar values** at the same level as the existing `alt_bn128_g1_multiplication_be`. Pippenger's window-based decomposition can leak through cache-timing if naive table lookups are used; the reference impl relies on arkworks's `VariableBaseMSM` which uses constant-time bucket accumulation. Production code paths in agave should match.

The validation surface is identical to `alt_bn128_g1_multiplication_be` (per-point: field + curve check, no subgroup check), so the same fuzz corpus applies. We recommend extending agave's existing alt_bn128 syscall fuzz harness with grouped-input MSM cases.

### Cost-model robustness

The per-point cost of 2,400 CU is derived from native Pippenger benchmarks at small to medium n. If a chosen implementation's per-point cost exceeds 2,400 at n = 4 (the smallest cost-effective MSM), the cost model should be raised to match or the implementation tuned. Anza's existing approach of measuring + landing concrete numbers via feature-gate is appropriate.

## Prior art

* [SIMD-0202 / 0207 / 0284](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0284-alt-bn128-little-endian.md): existing alt_bn128 G1 syscalls.
* [SIMD-0302](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0302-bn254-g2-syscalls.md): G2 syscalls — this SIMD's structural template.
* [EIP-2537](https://eips.ethereum.org/EIPS/eip-2537): BLS12-381 precompiles including G1/G2 MSM. Confirms that batched MSM is a recognised syscall-class primitive.
* arkworks `VariableBaseMSM::msm` (`ark-ec` crate): the canonical Pippenger implementation. Suitable as the reference for the agave native implementation.

## Reference implementation

* [`crates/g1-msm-ref`](https://github.com/nzengi/Solana-Plonk/tree/main/crates/g1-msm-ref) — pure-Rust no_std Pippenger MSM over BN254 G1. Used as the host-side oracle in this proposal's benchmarks; cross-checked against arkworks naive scalar-mul-and-add at n ∈ {0, 1, 2, 4, 8, 16, 32, 64} (11/11 tests pass).
* [`programs/g1-msm-bench`](https://github.com/nzengi/Solana-Plonk/tree/main/programs/g1-msm-bench) — Mollusk-driven CU benchmark grid. Source of all numbers in this document.

## Unresolved questions

1. **Maximum n**: should the syscall enforce a cap (e.g. n ≤ 256)? Larger n risks input-deserialisation DoS. Recommendation: enforce n ≤ 1024 with the cost model above; reject larger inputs with `InvalidInputData`.
2. **Strict-canonical scalar mode**: should there be a `_strict_be` variant that rejects non-canonical scalar encodings (≥ p)? Useful for verifiers that need strict input hygiene (groth16-solana rejects non-canonical proofs). Open for discussion.
3. **G2 MSM follow-up**: a corresponding `alt_bn128_g2_msm` could ship after this lands, scoped against the small-n range of typical pairing-equation aggregations. Out of scope here.

## Implementation tracking

Open an agave tracking issue under `programs/bpf_loader` once this SIMD is accepted, with the reference impl in [`crates/g1-msm-ref`](https://github.com/nzengi/Solana-Plonk/tree/main/crates/g1-msm-ref) as the starting point. Feature gate: TBD on activation.
