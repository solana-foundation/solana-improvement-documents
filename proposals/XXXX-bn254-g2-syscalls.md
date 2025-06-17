---
simd: '0302'
title: BN254 G2 Arithmetic Syscalls
authors:
  - Blockiosaurus (Metaplex Foundation)
category: Standard
type: Core
status: Review
created: 2025-06-12
feature:
supersedes:
superseded-by:
extends:
---

## Summary

Extend the existing `sol_alt_bn128_group_op` syscall to support native G2 curve
point arithmetic (addition, subtraction, and scalar multiplication) on the BN254
curve.

## Motivation

Solana today provides syscalls for Alt‑BN128 G1 group operations
(`ALT_BN128_ADD`, `ALT_BN128_SUB`, `ALT_BN128_MUL`) and for pairing checks, as
well as compression/decompression for G2 points. However, there is no direct
support for arithmetic on G2 points themselves, as they were not included in the
Ethereum Precompile on which the syscalls were based.

Adding native G2 syscalls will enable:

- Batch Groth16 verification: Multiple proofs could be aggregated into single
verification calls
- KZG polynomial commitments: Enable efficient multi-point opening proofs and
batch verification
- Other more advanced ZK systems for batch or direct verification

## New Terminology

- **G2 point**: A point on the twist subgroup of BN254, represented as two Fq₂
elements (each element is a pair of 32‑byte field coefficients).

## Detailed Design

### New Constants

```rust
// Field and point sizes
pub const ALT_BN128_FIELD_SIZE:       u64 = 32;
    // bytes per Fq element
pub const ALT_BN128_G2_POINT_SIZE:    u64 = ALT_BN128_FIELD_SIZE * 4;
    // x=(x0,x1), y=(y0,y1) each 32‑byte

// G2 addition/subtraction
pub const ALT_BN128_G2_ADDITION_INPUT_LEN:   u64 = ALT_BN128_G2_POINT_SIZE * 2;
    // 256
pub const ALT_BN128_G2_ADDITION_OUTPUT_LEN:  u64 = ALT_BN128_G2_POINT_SIZE;
    // 128

// G2 scalar multiplication
pub const ALT_BN128_G2_MULTIPLICATION_INPUT_LEN:  u64 = ALT_BN128_G2_POINT_SIZE
    + ALT_BN128_FIELD_SIZE; // 160
pub const ALT_BN128_G2_MULTIPLICATION_OUTPUT_LEN: u64 = ALT_BN128_G2_POINT_SIZE;
    // 128

### New Opcodes
pub const ALT_BN128_G2_ADD: u64 = 4;
pub const ALT_BN128_G2_SUB: u64 = 5;
pub const ALT_BN128_G2_MUL: u64 = 6;
```

### Endianness

Additional G2 operation support for little endian formats should also be
included use the little-endian encoding conventions as specified in
[SIMD-0284](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0284-alt-bn128-little-endian.md),
consistent with existing BN128 syscalls.

## Alternatives Considered

- Separate G2‑only syscall entrypoint rather than extending sol_alt_bn128_group_op.
- Drawback: Increases API surface, doubles maintenance.
- Library‑level intrinsics (BPF C/Rust) for G2 math as fallback.
- Drawback: Slower, higher compute cost than native syscalls.

## Impact

- Native G2 ops reduce compute units by approximately 10×–20× compared to
pure‑BPF implementations.
- Enables newer and more advanced ZK verification methods as well as batch proof
verification

## Security Considerations

None
