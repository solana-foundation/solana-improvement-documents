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

1. The addition, subtraction (negation) and scalar multiplication in G1 and G2
2. The pairing operation
3. The decompression operations in G1 and G2

Poseidon hash support is outside the scope of this SIMD.

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
([SIMD 326](https://github.com/solana-foundation/solana-improvement-documents/pull/326))
as well. Alpenglow consensus votes themselves do not require a BLS12-381
syscall as they are processed differently than regular transactions.

However, when a validator registers their BLS public key, it must do
so using a regular on-chain transaction that will pass through the normal
transaction pipeline. This transaction must contain a BLS Proof of Possession
(PoP) to validate ownership of the BLS public key. This PoP is a cryptographic
proof that is crucial to prevent a "rogue-key-attack", where a malicious
validator could try to register a key that is an algebraic combination of
their own key and a victim's key. This would allow the attacker to forge
aggregate signatures that appear to come from the entire group, even without the
victim's participation.

Initially, the vote program will natively execute PoP verification (see
[SIMD-0387](https://github.com/solana-foundation/solana-improvement-documents/pull/387)).
However, when the vote program eventually transitions to BPF, it will require a
way to efficiently verify PoP inside a BPF program. By introducing syscalls
for BLS12-381, a standard BPF program can efficiently verify these PoPs.

## New Terminology

- Proof of Possession (PoP): A cryptographic proof that an entity (e.g., a
  validator) holds the secret private key corresponding to a public key that they
  are attempting to register. In the context of BLS, this is typically a signature
  created with the private key over a message derived from the public key itself.

- Rogue-Key Attack: An attack against aggregate signature schemes (like BLS)
  where a malicious user registers a "rogue" public key that is mathematically
  constructed from their own key and one or more victim validators' public keys.
  This allows the attacker to forge aggregate signatures that appear to be validly
  signed by the entire group (including the victims), even without the victims'
  participation. Requiring a PoP at registration prevents this attack because the
  attacker cannot prove possession of the secret key for the rogue public key.

## Detailed Design

We propose the addition of the following operations on BLS12-381 as syscalls:

1. The addition, subtraction (negation) and scalar multiplication in G1 and G2
2. The pairing operation
3. The decompression operations in G1 and G2

These operations are sufficient to enable standard pairing-based cryptography
applications, for example, Groth16 proof verifier on BLS12-381 and BLS signature
or Proof of Possession (PoP) verifier. Additional operations are needed to
support more advanced ZK or cryptography applications, but these are outside the
scope of this SIMD.

### Curve Specification and Endianness (LE vs. BE)

For the curve definition, the
[IETF draft](https://www.ietf.org/archive/id/draft-irtf-cfrg-pairing-friendly-curves-11.html#name-bls-curves-for-the-128-bit-)
should be used as reference. For the encoding, the
[Zcash](https://github.com/zkcrypto/pairing/blob/34aa52b0f7bef705917252ea63e5a13fa01af551/src/bls12_381/README.md#serialization)
specification should be used. We note that the Zcash specification defines a
canonical big-endian (BE) encoding. To add the most flexibility, we also define
parallel little-endian (LE) variants.

Our LE variants mirror the Zcash standard in structure, with the only change
being the byte-ordering of the base field (Fq) elements themselves:

1. Fq Elements: A 381-bit field element is encoded in 48 bytes.
   - `_BE` variants expect this 48-byte array in big-endian.
   - `_LE` variants expect this 48-byte array in little-endian.

2. Fq^2 Element Ordering: An Fq2 element `(c0 + c1 * u)` is encoded as 96 bytes.
   The standard's structural ordering is preserved: the 48-byte encoding of the
   `c1` component (imaginary) comes first, followed by the 48-byte encoding of
   the `c0` component (real). Each 48-byte component respects the `_LE` or `_BE`
   flag.

3. Compressed Point Control Bits: For compressed representations (used by
   `sol_curve_decompress`), the Zcash standard uses the 3 most-significant-bits
   of the entire byte string (e.g., the first bits of the 48-byte G1 array) for
   flags (compression, infinity, and sign). This convention is retained for both
   BE and LE variants to ensure a consistent encoding structure. The syscall
   will always read these flags from the same most-significant bits of the byte
   array, while the remaining bits will be interpreted as the field element
   according to the specified endianness.

### Addition and Scalar Multiplication on G1 and G2

There is already a dedicated `sol_curve_group_op` syscall function for general
elliptic curve group operations. This function takes in a `curve_id`, `group_op`,
and two scalar/points that are encoded in little-endian. It interprets the
curve points according to the curve id, and applies the group operations
specified by the `group_op`. Currently, the syscall supports the curve25519
edwards and ristretto representations.

This function can be extended to support the addition, subtraction, and scalar
multiplication in BLS12-381 G1 and G2. We propose adding new `curve_id` constants
for BLS12-381. The syscall will interpret inputs differently based on the
`curve_id`:

- BLS12-381 inputs (using `BLS12_381_G1_{BE,LE}`, `BLS12_381_G2_{BE,LE}`)
  will be interpreted as points in affine representation in either little-endian
  or big-endian.
- Curve25519 inputs (using `CURVE25519_EDWARDS` or `CURVE25519_RISTRETTO`) are
  interpreted as points in their respective Edwards or Ristretto representations
  in compressed little-endian representations.

```rust
pub const CURVE25519_EDWARDS: u64 = 0;
pub const CURVE25519_RISTRETTO: u64 = 1;

// Reserve indices 2 and 3 in case we want to support affine representations of
// curve25519 points in the future
// pub const CURVE25519_EDWARDS_AFFINE_LE: u64 = 2;
// pub const CURVE25519_EDWARDS_AFFINE_BE: u64 = 2 | 0x80;
// pub const CURVE25519_RISTRETTO_AFFINE_LE: u64 = 3;
// pub const CURVE25519_RISTRETTO_AFFINE_BE: u64 = 3 | 0x80;

// New Curve ID
pub const BLS12_381_LE: u64 = 4;
pub const BLS12_381_BE: u64 = 4 | 0x80;
pub const BLS12_381_G1_LE: u64 = 5;
pub const BLS12_381_G1_BE: u64 = 5 | 0x80;
pub const BLS12_381_G2_LE: u64 = 6;
pub const BLS12_381_G2_BE: u64 = 6 | 0x80;

pub const ADD: u64 = 0;
pub const SUB: u64 = 1;
pub const MUL: u64 = 2;
```

The `BLS12-381` constant will be used for the pairing operation.
The `BLS12_381_G1` and `BLS12_381_G2` constants will be used for group operations
and decompression.

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
points in G2 to support batch pairings.

```rust
define_syscall!(fn sol_curve_pairing_map(
    curve_id: u64,
    num_pairs: u64,
    g1_points: *const u8,
    g2_points: *const u8,
    result: *mut u8
) -> u64);
```

The `num_pairs` parameter specifies the number of G1/G2 pairs to be processed.
The `g1_points` and `g2_points` parameters are pointers to memory buffers.

The syscall will read `num_pairs` from each buffer. It must read:

1. `num_pairs * 96` bytes from the `g1_points` buffer (96 bytes per uncompressed
   affine G1 point).
2. `num_pairs * 192` bytes from the `g2_points` buffer (192 bytes per
   uncompressed affine G2 point).
   If the `num_pairs` is 0, the syscall should return success with the identity
   element of the target group.

The runtime must safely handle memory accesses. If reading the required number
of bytes from either buffer results in an out-of-bounds memory access (e.g., the
buffers are smaller than implied by `num_pairs`), the syscall must return an
error.

The function would interpret `g1_points` as an array of BLS12-381 points in G1
`[P1, ..., Pn]` and `g2_points` as an array of BLS12-381 points in G2
`[Q1, ..., Qn]`. Both inputs are interpreted as affine representations
encoded in either little-endian or big-endian depending on the curve id. It
should then compute the pairing product `e(P1, Q1) * ... * e(Pn, Qn)`.
The result of the syscall will be the actual target group element from the
product of pairings.

### Decompression Operations in G1 and G2

For the decompression operations in G1 and G2, we propose adding a dedicated
syscall for general decompression `sol_curve_decompress`.

```rust
define_syscall!(fn sol_curve_decompress(
    curve_id: u64,
    point: *const u8,
    result: *mut u8
) -> u64);
```

This function will take in a curve id and a point that is represented in its
compressed representation (encoded in little-endian or big-endian). It will
decompress the compressed point into an affine representation and write the
result.

This function must perform a full point validation as specified in the
[Zcash](https://github.com/zkcrypto/pairing/blob/34aa52b0f7bef705917252ea63e5a13fa01af551/src/bls12_381/README.md#serialization)
specification. For the operation to succeed, the input bytes must pass all of
the following checks:

1. Format Check: The control bits (compression, infinity, and sign) in the
   most-significant bits are valid as per the Zcash standard.
2. Field Check: The `x`-coordinate is a valid field element (i.e., less than the
   modulus `p`).
3. On-Curve Check: The point satisfies the curve equation.
4. Subgroup Check: The resulting point is in the correct prime-order subgroup
   `r`.

## Alternatives Considered

This proposal extends the existing `sol_curve_group_op` and
`sol_curve_pairing_map` syscalls to add support for BLS12-381. The main design
alternative would be to add separate dedicated syscall functions for BLS12-381
as was done for the BN254 curve.

We chose to extend the existing syscalls for few reasons:

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

The necessary security considerations such as point validation, memory safety,
etc. are detailed in the Detailed Design section.
