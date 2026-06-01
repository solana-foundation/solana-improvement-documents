---
simd: '0302'
title: BN254 G2 Arithmetic Syscalls
authors:
  - Blockiosaurus (Metaplex Foundation)
  - sls_0x (Parad0x Labs)
category: Standard
type: Core
status: Draft
created: 2025-06-12
feature:
supersedes:
superseded-by:
extends:
---

## Summary

Extend the existing `sol_alt_bn128_group_op` syscall to support native G2
curve point arithmetic (addition, subtraction, and scalar multiplication)
on the BN254 curve.

## Motivation

Solana today provides syscalls for Alt-BN128 G1 group operations
(`ALT_BN128_ADD`, `ALT_BN128_SUB`, `ALT_BN128_MUL`) and for pairing
checks, as well as compression/decompression for G2 points. However,
there is no direct support for arithmetic on G2 points themselves, as
they were not included in the Ethereum Precompile on which the syscalls
were based.

Adding native G2 syscalls will enable:

- Batch Groth16 verification: Multiple proofs could be aggregated into
  single verification calls
- KZG polynomial commitments: Enable efficient multi-point opening
  proofs and batch verification
- Other more advanced ZK systems for batch or direct verification

## New Terminology

- **G2 point**: A point on the twist subgroup of BN254, represented as
  two Fq2 elements (each element is a pair of 32-byte field
  coefficients).

## Detailed Design

### New Constants

```rust
// Field and point sizes
pub const ALT_BN128_FIELD_SIZE:    u64 = 32;
    // bytes per Fq element
pub const ALT_BN128_G2_POINT_SIZE: u64 = ALT_BN128_FIELD_SIZE * 4;
    // x=(x0,x1), y=(y0,y1) each 32-byte

// G2 addition/subtraction
pub const ALT_BN128_G2_ADDITION_INPUT_LEN:   u64 =
    ALT_BN128_G2_POINT_SIZE * 2; // 256
pub const ALT_BN128_G2_ADDITION_OUTPUT_LEN:  u64 =
    ALT_BN128_G2_POINT_SIZE;     // 128

// G2 scalar multiplication
pub const ALT_BN128_G2_MULTIPLICATION_INPUT_LEN:  u64 =
    ALT_BN128_G2_POINT_SIZE + ALT_BN128_FIELD_SIZE; // 160
pub const ALT_BN128_G2_MULTIPLICATION_OUTPUT_LEN: u64 =
    ALT_BN128_G2_POINT_SIZE; // 128
```

### New Opcodes

```rust
pub const ALT_BN128_G2_ADD: u64 = 4;
pub const ALT_BN128_G2_SUB: u64 = 5;
pub const ALT_BN128_G2_MUL: u64 = 6;
```

### Endianness

Additional G2 operation support for little-endian formats should also
be included, using the little-endian encoding conventions as specified
in [SIMD-0284], consistent with existing BN128 syscalls.

[SIMD-0284]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0284-alt-bn128-little-endian.md

### Validation and Subgroup Checks

The validation requirements for the new G2 operations are defined as
follows to balance safety and compute cost, consistent with the approach
for BLS12-381:

1. _G2 Addition (`ALT_BN128_G2_ADD`) and Subtraction
   (`ALT_BN128_G2_SUB`)_:
   - Input points MUST be validated to ensure they are on the curve
     (satisfy the G2 curve equation).
   - The _subgroup check is skipped_. The addition formulas are valid
     for any point on the curve (including those not in the prime-order
     subgroup). Skipping this costly check allows for cheaper
     accumulation of points.

2. _G2 Scalar Multiplication (`ALT_BN128_G2_MUL`)_:
   - Input points MUST undergo _full validation_, including the field
     check, curve equation check, and the _subgroup check_.
   - Enforcing the subgroup check is required to safely support faster
     endomorphism-based scalar multiplication algorithms.

_Note on G1_: For the existing G1 operations, the BN254 (Alt-BN128)
curve has a cofactor of 1. This means that every point satisfying the
curve equation is automatically in the prime-order subgroup. Therefore,
no separate subgroup check is needed for G1 as the on-curve check is
sufficient.

## Alternatives Considered

- Separate G2-only syscall entrypoint rather than extending
  `sol_alt_bn128_group_op`.
  - Drawback: Increases API surface, doubles maintenance.
- Library-level intrinsics (BPF C/Rust) for G2 math as fallback.
  - Drawback: Slower, higher compute cost than native syscalls.

## Impact

- Native G2 ops reduce compute units by approximately 10x-20x compared
  to pure-BPF implementations.
- Enables newer and more advanced ZK verification methods as well as
  batch proof verification.

## Security Considerations

None.

---

## Revival note (2026-06-01)

Revived by sls_0x (Parad0x Labs) as implementation champions.

### Why we care

Parad0x Labs ships the `dark_bn254_gate` program on Solana mainnet
(program ID: `GCptvBYF8S6eVYoh15B7WAESc54FUHCpN1Ui6aHeQYZd`). It uses
the existing G1 syscalls (`ALT_BN128_ADD`, `ALT_BN128_MUL`,
`ALT_BN128_PAIRING`) to verify BN254-based ZK proofs on-chain.

Running a complete Groth16 verifier requires arithmetic on both G1 and
G2 points. Specifically, the verification key for a Groth16 proof
includes `beta_g2` and `gamma_g2` — verification key points that live
in G2. The final pairing check computes:

```
e(A, B) * e(alpha_g1, neg(beta_g2)) * e(L, neg(gamma_g2)) == 1
```

`alpha_g1` and `L` are G1 points and can be computed with existing
syscalls. `beta_g2` and `gamma_g2` are G2 points. Without native G2
arithmetic, an on-chain Groth16 verifier must hardcode those constants
at deploy time and cannot support variable verification keys. The same
constraint applies to KZG-based proof systems (PLONK, Marlin) that use
G2 commitments.

### What we commit to

1. **Reference implementation** — We will write a reference BPF program
   exercising `ALT_BN128_G2_ADD`, `ALT_BN128_G2_SUB`, and
   `ALT_BN128_G2_MUL` against known test vectors from the EIP-2537 and
   EIP-196/197 test suites.
2. **Test vectors** — We will provide a comprehensive set of test
   vectors (valid points, infinity, out-of-subgroup inputs) covering
   both big-endian and little-endian encodings per SIMD-0284.
3. **Anza coordination** — We will open tracking issues in
   `anza-xyz/agave` to align on compute-unit budgets and syscall
   activation gating once the spec is accepted.
4. **Compute-unit benchmarks** — We will benchmark the proposed
   opcodes against our existing mainnet program to confirm the
   10x-20x CU reduction claim in the Impact section above.
