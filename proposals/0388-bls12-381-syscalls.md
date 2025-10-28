---
simd: "0388"
title: BLS12-381 Elliptic Curve Syscalls
authors: Sam Kim (Anza)
category: Standard
type: Core
status: Review
created: 2025-10-28
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal introduces a new family of syscalls to provide native support for
cryptographic operations on the BLS12-381 elliptic curve. These syscalls will
expose:

1. The addition, subtraction (negation) and scalar multiplication in G1
2. The pairing operation
3. The decompression operations in G1 and G2

Group operations on G2 and Poseidon hash support is outside the scope of this
SIMD for now.

## Motivation

Solana's current support for pairing-based cryptography is limited to the
`alt_bn128` (also known as BN254) curve. Pairing-friendly curves are the
foundation for many efficient and modern zero-knowledge proof systems, such as
those based on Groth16. While the BN254 curve enables basic cryptography and
zero-knowledge applications, it does not provide a 128-bit security level, which
is insufficient for high-security protocols.

The BLS12-381 curve is a modern, widely adopted standard for a pairing-friendly
curve that achieves a 128-bit security level. It is adopted in other major
ecosystems, such as Ethereum, which already supports it via a precompile.

Adding support for the BLS12-381 curve is important for enabling Alpenglow consensus
as well. Alpenglow intends to process standard vote transactions by verifying
BLS signatures in a dedicated BLS sigverify stage within the validator logic, so
a BLS12-381 syscall is not necessary for the vote transactions themselves.

However, when a validator registers their BLS public key, it must do so using a
regular on-chain transaction that will pass through the normal transaction
pipeline. This transaction must contain a BLS Proof of Possession (PoP) to
validate ownship of the BLS public key. This PoP is a cryptographic proof that
is crucial to prevent a "rogue-key-attack", where a malicious validator could,
roughly speaking, try to register another validator's public key as their own.

By introducing syscalls for BLS12-381, a standard BPF program can efficiently
verify these Proofs-of-Possessions.

## New Terminology

NA

## Detailed Design

We propose the addition of the following operations on BLS12-381 as syscalls:

1. The addition, subtraction (negation) and scalar multiplication in G1
2. The pairing operation
3. The decompression operations in G1 and G2

These operations are sufficient to enable standard pairing-based cryptography
applications, for example, Groth16 proof verifier on BLS12-381 and BLS signature
or Proof Of Possession (PoP) verifier. Additional operations are needed to
support more advanced ZK or cryptography applications, but these are outside the
scope of this SIMD for now.

### Addition and Scalar Multiplication on G1

There is already a dedicated `sol_curve_group_op` syscall function for general
elliptic curve group operations. This function takes in a `curve_id`, `group_op`,
and two scalar/points that are encoded in little-endian. It interprets the
curve points according to the curve id, and applies the group operations
specified by the `group_op`. Currently, the syscall supports the curve25519
edwards and ristretto representations.

This function can be extended to support the addition, subtraction, and scalar
multiplication in BLS12-381 G1. We propose adding new `curve_id` constants for
BLS12-381. The BLS12-381 inputs should be interpreted as affine points unlike
how Curve25519 points are currently interpreted by the syscall.

```rust
pub const CURVE25519_EDWARDS: u64 = 0;
pub const CURVE25519_RISTRETTO: u64 = 1;

// New Curve ID
pub const BLS12_381: u64 = 2;
pub const BLS12_381_G1: u64 = 3;
pub const BLS12_381_G2: u64 = 3;

pub const ADD: u64 = 0;
pub const SUB: u64 = 1;
pub const MUL: u64 = 2;
```

The `BLS12_381_G1` and `BLS12_381_G2` constants will be used for group
operations and decompression on the respective groups. The `BLS12_381` constant
will be used for the pairing operations which invovles both.

### Pairing Operation

There is an existing definition for a `sol_curve_pairing_map` syscall function for
elliptic curve pairing operations.

```rust
define_syscall!(fn sol_curve_pairing_map(
    curve_id: u64,
    point: *const u8,
    result: *mut u8
) -> u64);
```

This function is not actually instantiated at the moment. We propose updating
this function's signature to take in an array of points in G1 and an array of
pointst in G2 to support batch pairings.

```rust
define_syscall!(fn sol_curve_pairing_map(
    curve_id: u64,
    g1_points: *const u8,
    g2_points: *const u8,
    result: *mut u8
) -> u64);
```

The function would interpret `g1_points` as an array of BLS12-381 points in G1
`[P1, ..., Pn]` and `g2_points` as an array of BLS12-381 points in G2
`[Q1, ..., Qn]`. Both inputs are interpreted as affine representations
encoded in little-endian. It should then compute the pairing product
`e(P1, Q1) * ... * e(Pn, Qn)`.

### Decompression Operations in G1 and G2

For the decompression operations in G1 and G2, we propose adding a dedicated
syscall for general decompression `sol_curve_decompress`.

```rust
define_syscall!(fn sol_curve_pairing_map(
    curve_id: u64,
    point: *const u8,
    result: *mut u8
) -> u64);
```

This function will take in a curve id and a point that is represented in its
compressed representation (encoded in little-endian). It will decompress the
compressed point into an affine representation and write the result.

## Alternatives Considered

This proposal extends the existing `sol_curve_group_op` and
`sol_curve_pairing_map` syscalls to add support for BLS12-381. The main design
alternative would be to add separate dedicated syscall functions for BLS12-381
as was done for the BN254 curve.

We chose to extend the existing syscalls for a few reasons:

1. It follows the established pattern. The `sol_curve_group_op` syscall was
   designed to be extensible, using a `curve_id` to handle different curves like
   `CURVE25519_EDWARDS` and `CURVE25519_RISTRETTO`. Adding a new `curve_id` for
   BLS12-381 is a consistent and logical way to extend it.

2. It avoids syscall bloat. New curves will keep coming and if we add a new,
   dedicated syscall for every single one, that would make the runtime hard to
   maintain and audit. A generic interface is cleaner.

3. It's a simpler and safer code change. Adding a new `curve_id` to an existing
   `match` statement is a small, localized, and low-risk change. Plumbing an
   entirely new syscall is a much more complex and error-prone task.

## Impact

This will enable ZK and other more advanced pairing-based cryptography applications
to be built on Solana based on a more secure and modern curve.

## Security Considerations

NA
