---
simd: "0565"
title: secp256r1 (NIST P-256) Curve Syscalls
authors:
  - SK, ZZ
category: Standard
type: Core
status: Draft
created: 2026-06-16
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal introduces support for the `secp256r1` (also known as NIST P-256)
elliptic curve to Solana's existing generalized curve syscalls. These syscalls
will expose:

1. Group operations (addition, subtraction/negation, and scalar multiplication)
2. Multiscalar multiplication (MSM)
3. Point validation
4. Decompression operations

By extending existing low-level syscalls rather than introducing a monolithic
signature verification syscall, this SIMD enables generalized cryptographic
operations on the `secp256r1` curve directly within BPF programs.

## Motivation

There is an ongoing effort within the Solana ecosystem to migrate cryptographic
precompiles from the validator client into standard BPF programs. Currently, the
`ed25519` and `secp256k1` signature verifications can be implemented natively
and cost-effectively in BPF using Solana's existing syscalls. However,
`secp256r1` lacks this underlying syscall support, making efficient BPF
implementations for it expensive. Extending the existing curve syscalls to
include `secp256r1` resolves the missing capability.

Outside the context of precompiles, the `secp256r1` curve (also known as NIST
P-256) is a widely adopted industry standard. It is the primary curve utilized
by Apple's Secure Enclave, Android Keystore, WebAuthn, and Passkeys. Equipping
developers with native, low-level support for `secp256r1` provides a  
bridge between traditional hardware security modules and on-chain accounts,
significantly expanding the cryptographic toolkit available on Solana.

## New Terminology

- secp256r1 / NIST P-256: A widely used elliptic curve standardized by NIST.
  Unlike `secp256k1` (used in Bitcoin and Ethereum), `secp256r1` is the standard
  for hardware-backed security modules (Secure Enclave, TPMs) and WebAuthn.

## Detailed Design

We propose extending the existing `sol_curve_*` syscall family to support
`secp256r1`. The required operations are:

1. Group operations (addition, subtraction, and scalar multiplication)
2. Multiscalar multiplication (MSM)
3. Point validation
4. Decompression operations

### Curve Specification and Encoding

For the curve definition, the standard SP 800-186 specifications for NIST P-256
should be used.

To maintain consistency with existing curve syscalls, points are serialized as
fixed-size 64-byte arrays representing uncompressed affine coordinates. This
consists of 32 bytes for the X-coordinate, immediately followed by 32 bytes for
the Y-coordinate.

The point at infinity (the identity element) is canonically encoded as 64 bytes
of zeroes. Because `(0, 0)` is not a valid point on the `secp256r1` curve, this
provides a safe, fixed-length representation of the identity point.

To provide flexibility across different application architectures, we define
both little-endian (LE) and big-endian (BE) variants for scalar and field
element byte-ordering within these fixed-length arrays.

We note that this deviates from the standard SEC1 encoding. SEC1 is a
variable-length format (e.g., representing the identity point as a single `0x00`
byte), making it more error-prone for validator clients to handle.

The exception to this rule is the `sol_curve_decompress` syscall. This syscall
accepts a fixed 33-byte compressed structure (a 1-byte parity prefix of 0x02 or
0x03 followed by a 32-byte X-coordinate) and translates it into the 64-byte
uncompressed representation.

The byte-ordering of the 32-byte input X-coordinate, as well as the resulting
64-byte uncompressed output, is strictly determined by the endianness specified
by the `curve_id` (`SECP256R1_LE` or `SECP256R1_BE`). Note that when using the
`LE` identifier, the input X-coordinate must be little-endian, which deviates
from the natively big-endian standard SEC1 format.

### New Curve ID Constants

We propose using new `curve_id` constants for `secp256r1`. These IDs will be
used across the extended syscalls to specify the curve and endianness.

```rust
pub const CURVE25519_EDWARDS: u64 = 0;
pub const CURVE25519_RISTRETTO: u64 = 1;

// indices 2 and 3 reserved in case we want to support curve25519 affine

pub const BLS12_381_LE: u64 = 4;
pub const BLS12_381_BE: u64 = 4 | 0x80;
pub const BLS12_381_G1_LE: u64 = 5;
pub const BLS12_381_G1_BE: u64 = 5 | 0x80;
pub const BLS12_381_G2_LE: u64 = 6;
pub const BLS12_381_G2_BE: u64 = 6 | 0x80;

// New secp256r1 Curve IDs
pub const SECP256R1_LE: u64 = 7;
pub const SECP256R1_BE: u64 = 7 | 0x80;
```

### Addition and Scalar Multiplication

The existing `sol_curve_group_op` syscall will be extended to support
`secp256r1`.

```rust
define_syscall!(fn sol_curve_group_op(
    curve_id: u64,
    group_op: u64,
    left_input_addr: *const u8,
    right_input_addr: *const u8,
    result_point_addr: *mut u8
) -> u64);
```

The syscall will use the newly defined `SECP256R1_{LE,BE}` constants. The
group_op parameter will use the existing `ADD`, `SUB`, and `MUL` constants.

Validation rules for `secp256r1` operands:

1. Addition (`ADD`) and Subtraction (`SUB`): Both input points must undergo full
   validation (field check, curve equation check). If either input point is
   invalid, the syscall must immediately fail and return `1`. If the operation
   succeeds and results in the point at infinity (e.g., `P - P`), the syscall
   must write the canonical 64-byte all-zero array to `result_point_addr` and
   return `0`. Otherwise, it writes the resulting affine point to
   `result_point_addr` and returns `0`.

2. Scalar Multiplication (`MUL`): The point must undergo full validation (field
   check, curve equation check). The scalar input must also be validated to
   ensure it is strictly less than the curve's prime group order `n`. If the
   scalar is out of range, or if point validation fails, the syscall must
   immediately fail and return `1`. Since `secp256r1` has a cofactor of 1, any
   point on the curve is inherently in the correct prime-order subgroup, making
   subgroup checks mathematically trivial. Multiplying by a scalar of `0` must
   yield the canonical 64-byte all-zero array.

### Multiscalar Multiplication (MSM)

The existing `sol_curve_multiscalar_mul` syscall will be extended for
`secp256r1`.

```rust
define_syscall!(fn sol_curve_multiscalar_mul(
    curve_id: u64,
    scalars_addr: *const u8,
    points_addr: *const u8,
    points_len: u64,
    result_point_addr: *mut u8
) -> u64);
```

This extension is useful for batch-verifying WebAuthn signatures or evaluating
complex cryptographic equations efficiently. The implementation should leverage
sub-linear optimization algorithms (e.g., Pippenger's algorithm) to reduce
compute units (CU) compared to sequential scalar multiplications.

Validation rules for MSM operands:

1. Every point in the `points_addr` array must undergo full validation (field
   check, curve equation check).
2. Every scalar in the `scalars_addr` array must be validated to ensure it is
   strictly less than the curve's prime group order `n`.

If any point is invalid, or if any scalar is out of range, the syscall must
immediately fail and return `1` without writing to `result_point_addr`. If the
final sum results in the point at infinity, the syscall must write the canonical
64-byte all-zero array and return `0`. Otherwise, it writes the resulting affine
point and returns `0`.

### Point Validation

The `sol_curve_validate_point` syscall will be extended to support `secp256r1`.

```rust
define_syscall!(fn sol_curve_validate_point(
    curve_id: u64,
    point_addr: *const u8,
    result: *mut u8
) -> u64);
```

When called with `SECP256R1_{LE,BE}`, the syscall reads the uncompressed affine
coordinates (64 bytes: 32 bytes for X, 32 bytes for Y) from `point_addr`.

The validation succeeds (returning `0`) if one of the following conditions is
met:

1. Identity Point: The input is exactly 64 bytes of zeroes (the canonical
   representation of the point at infinity).
2. On-Curve Point: The coordinates are valid field elements (strictly less than
   the prime modulus `p`) and satisfy the curve equation.

If neither condition is met (e.g. either coordinate is `>= p`, or the point does
not lie on the curve), the syscall must return `1`.

### Decompression Operations

The `sol_curve_decompress` syscall will be extended for `secp256r1`.

```rust
define_syscall!(fn sol_curve_decompress(
    curve_id: u64,
    point: *const u8,
    result: *mut u8
) -> u64);
```

It accepts a 33-byte compressed `secp256r1` point (a 1-byte parity marker of
`0x02` or `0x03` and a 32-byte X-coordinate). The endianness of both the input
X-coordinate and the output 64-byte affine representation to `result` is
dictated by the `curve_id` used. It calculates the valid Y-coordinate and
outputs the standard uncompressed representation.

Because the point at infinity has no affine X-coordinate and is represented in
standard SEC1 as a single `0x00` byte, it cannot be encoded within the fixed
33-byte layout required by this syscall. As such, decompression of the identity
point is explicitly unsupported.

The provided X-coordinate must be a valid field element (strictly less than the
prime modulus `p`). The syscall must fail and return `1` if any of the following
conditions are met:

1. The X-coordinate is `>= p`.
2. The 1-byte prefix marker is not `0x02` or `0x03`.
3. The calculated `Y^2` value is a quadratic non-residue.

On success, the syscall writes the standard uncompressed 64-byte affine
representation to `result` and returns `0`.

## Alternatives Considered

Add a monolithic `sol_secp256r1_verify` or `sol_secp256r1_recover` syscall.
While a single syscall might be easier to integrate for a narrow set of
signature verification use cases, it limits developer freedom. A monolithic
verify syscall forces the network to enshrine specific rules around signature
malleability (e.g., enforcing the rejection of high-S values). It also precludes
developers from using the curve for anything other than standard ECDSA
verification. Extending the generalized curve operations follows the established
architectural direction of the network.

## Impact

This will support the strategic migration of cryptographic precompiles to Core
BPF and enable hardware-secured WebAuthn and Passkey applications to be built
natively on Solana.

## Security Considerations

The necessary security considerations such as point validation, memory safety,
etc. are detailed in the Detailed Design section.

To prevent Denial-of-Service (DoS) attacks, the Compute Unit (CU) cost for each
`secp256r1` syscall must strictly align with the actual wall-clock execution
time on validator hardware. If these cryptographic operations are underpriced,
an attacker could exploit the cost tracker inaccuracy to artificially delay
block production while remaining ostensibly under the 60 million CU block limit.

Furthermore, the `sol_curve_multiscalar_mul` syscall requires a dynamic CU
pricing model. Because it utilizes sub-linear optimization algorithms (e.g.,
Pippenger's algorithm), the computational cost does not scale linearly with
respect to `points_len`. The cost tracker must accurately reflect this specific
sub-linear scaling curve to prevent overcharging users for large batch
operations, while still mathematically guaranteeing the validator cannot be
stalled.
