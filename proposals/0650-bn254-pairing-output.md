---
simd: 'XXXX'
title: BN254 Pairing with Target Group Output
authors:
  - SK, ZZ (Anza)
category: Standard
type: Core
status: Idea
created: 2026-09-20
feature: (fill in with feature key and github tracking issues once accepted)
extends: SIMD-0302
---

## Summary

Add a new opcode, `ALT_BN128_PAIRING_MAP`, to the existing
`sol_alt_bn128_group_op` syscall. It computes the same product of pairings on
the BN254 curve as the existing `ALT_BN128_PAIRING` opcode, but writes the
resulting target group (Gt) element instead of a 32-byte identity flag. No
existing opcode, input format, or output format is changed.

## Motivation

The existing BN254 pairing operation (`ALT_BN128_PAIRING`) mirrors the Ethereum
precompile: it interprets its input as an array of `(G1, G2)` pairs, computes
the product of pairings `e(P1, Q1) * ... * e(Pn, Qn)`, and reports only whether
the product equals the identity of the target group. The target group element
itself is never exposed.

This is sufficient for Groth16-style verification, which can be rearranged
into a single identity check, but it rules out applications that need the
actual pairing output:

- Comparing the pairing product against a target group element computed
  off-chain and stored on-chain, instead of re-deriving it every time
- Splitting an expensive verification across several transactions, by computing
  partial pairing products in separate syscalls and combining them later
- Identity-based and time-lock encryption, where the pairing output is used
  directly as key material (e.g. `H(e(P, Q))`)
- Feature parity with the BLS12-381 syscalls of
  [SIMD-0388](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0388-bls12-381-syscalls.md),
  whose pairing operation returns the target group element

## Dependencies

- **[SIMD-0284]: Alt-BN128 Little Endian compatibility**

    Defines the `0x80` opcode flag that selects little-endian encodings for
    the BN254 syscalls. The new opcode follows the same convention.

- **[SIMD-0302]: BN254 G2 Arithmetic Syscalls**

    Allocates opcodes `4`, `5`, and `6` of `sol_alt_bn128_group_op`. This
    proposal allocates the next free opcode, `7`.

- **[SIMD-0334]: Fix alt_bn128_pairing syscall length check**

    Defines the input length check that the new opcode reuses.

[SIMD-0284]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0284-alt-bn128-little-endian.md
[SIMD-0302]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0302-bn254-g2-syscalls.md
[SIMD-0334]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0334-fix-alt-bn128-pairing-length-check.md

## New Terminology

- **Target group (Gt) element**: The output of the BN254 pairing, an element of
  the order-`r` subgroup of the degree-12 extension field Fq12, represented as
  twelve 32-byte base field coefficients.

## Detailed Design

The key words "MUST", "MUST NOT", "SHOULD", and "MAY" in this document are to
be interpreted as described in [RFC 2119](https://www.ietf.org/rfc/rfc2119.txt)
and [RFC 8174](https://www.ietf.org/rfc/rfc8174.txt).

The new operation is added to the existing syscall, whose signature is
unchanged:

```rust
define_syscall!(fn sol_alt_bn128_group_op(
    group_op: u64,
    input: *const u8,
    input_size: u64,
    result: *mut u8
) -> u64);
```

### New Constants and Opcode

```rust
pub const ALT_BN128_FIELD_SIZE:    u64 = 32;                         // Fq
pub const ALT_BN128_G1_POINT_SIZE: u64 = ALT_BN128_FIELD_SIZE * 2;   // 64
pub const ALT_BN128_G2_POINT_SIZE: u64 = ALT_BN128_FIELD_SIZE * 4;   // 128
pub const ALT_BN128_GT_ELEMENT_SIZE: u64 = ALT_BN128_FIELD_SIZE * 12; // 384

pub const ALT_BN128_PAIRING_MAP_ELEMENT_LEN: u64 = ALT_BN128_G1_POINT_SIZE
    + ALT_BN128_G2_POINT_SIZE; // 192, same as ALT_BN128_PAIRING_ELEMENT_LEN
pub const ALT_BN128_PAIRING_MAP_OUTPUT_LEN:  u64 = ALT_BN128_GT_ELEMENT_SIZE;
    // 384

// Existing opcodes, unchanged:
//   ALT_BN128_ADD = 0, ALT_BN128_SUB = 1, ALT_BN128_MUL = 2,
//   ALT_BN128_PAIRING = 3,
//   ALT_BN128_G2_ADD = 4, ALT_BN128_G2_SUB = 5, ALT_BN128_G2_MUL = 6
pub const ALT_BN128_PAIRING_MAP: u64 = 7;
```

### Endianness

Following SIMD-0284, `ALT_BN128_PAIRING_MAP` (`7`) selects the big-endian
(EIP-197) encoding and `ALT_BN128_PAIRING_MAP | 0x80` selects the little-endian
encoding. The flag applies to both the input points and the output target group
element (see [Target Group (Gt) Encoding](#target-group-gt-encoding)).

### Input

The input layout is identical to that of `ALT_BN128_PAIRING`, so existing
callers can switch opcodes without re-encoding their data. The input is a
sequence of `ALT_BN128_PAIRING_MAP_ELEMENT_LEN` (192) byte elements, each an
uncompressed affine G1 point (64 bytes) immediately followed by an uncompressed
affine G2 point (128 bytes). Points are encoded exactly as for the
corresponding `ALT_BN128_PAIRING` variant.

`input_size` MUST be a multiple of 192; otherwise the syscall MUST return an
error, consistent with SIMD-0334.

The number of pairs is not capped by the syscall itself. As for
`ALT_BN128_PAIRING`, compute units are charged per pair, so the transaction
compute budget bounds the amount of work.

### Input Validation

Validation MUST be identical to that of the existing `ALT_BN128_PAIRING` opcode
at the time the feature is activated. In particular:

1. Every field element MUST be canonical (less than the base field modulus).
2. G1 points MUST satisfy the curve equation. The BN254 curve has a cofactor of
   1 in G1, so every point on the curve is in the prime-order subgroup and no
   separate subgroup check is needed.
3. G2 points MUST satisfy the curve equation and MUST be in the prime-order
   subgroup. The pairing is only well defined on the prime-order subgroup, so
   this check cannot be skipped.
4. The all-zero encoding is accepted as the point at infinity for both G1 and
   G2.

If any input fails validation, the syscall MUST return an error and MUST NOT
write to the output buffer.

### Output

On success, the syscall writes exactly `ALT_BN128_PAIRING_MAP_OUTPUT_LEN` (384)
bytes to `result`: the target group element `e(P1, Q1) * ... * e(Pn, Qn)` after
the final exponentiation, encoded as specified below. The element is fully
reduced and canonical, so equal target group elements always have
byte-identical encodings and callers MAY compare outputs by comparing bytes.

If `input_size` is `0` (zero pairs), the syscall MUST succeed and write the
multiplicative identity of the target group. This matches the existing
`ALT_BN128_PAIRING` behavior of returning `1` for an empty input, and the
BLS12-381 pairing syscall.

### Target Group (Gt) Encoding

The target group is a subgroup of Fq12, represented as a tower of extensions in
the same way as for BLS12-381 in SIMD-0388:

1. `Fq12 = c0 + c1 * w` is a quadratic extension of `Fq6`, with `w^2 = v`
2. `Fq6 = c0 + c1 * v + c2 * v^2` is a cubic extension of `Fq2`, with
   `v^3 = 9 + u`
3. `Fq2 = c0 + c1 * u` is a quadratic extension of `Fq`, with `u^2 = -1`

This is the tower used by `ark-bn254` and by most BN254 libraries.

Flattening the tower yields twelve base field coefficients `c0, ..., c11`,
where the coefficient `Fq12.c_a.c_b.c_c` (with `a, c ∈ {0, 1}` and
`b ∈ {0, 1, 2}`) is `c_(6a + 2b + c)`. That is, coefficients are listed in
canonical memory order: the `Fq6` component `c0` of `Fq12` first, within each
`Fq6` the `Fq2` components `c0`, `c1`, `c2` in order, and within each `Fq2` the
real part before the imaginary part.

The 384-byte target group element is serialized as follows:

1. **Little-Endian (`ALT_BN128_PAIRING_MAP | 0x80`)**: coefficients in the
   order `c0, c1, ..., c11`, each 32-byte `Fq` coefficient in little-endian.
2. **Big-Endian (`ALT_BN128_PAIRING_MAP`)**: coefficients in the order
   `c11, c10, ..., c0`, each 32-byte `Fq` coefficient in big-endian.

The big-endian encoding is exactly the byte-reversal of the little-endian
encoding. This is the rule SIMD-0388 uses for BLS12-381, and it is consistent
with how the existing BN254 big-endian (EIP-197) encoding orders `Fq2`
elements: reversing the 64-byte little-endian encoding `(c0, c1)` of an `Fq2`
element yields the big-endian encoding `(c1, c0)`, imaginary part first.

For example, the multiplicative identity `1` of the target group is encoded as:

- **LE**: `01 00 ... 00` (`0x01` followed by 383 zero bytes), i.e. `c0 = 1`
  and all other coefficients `0`.
- **BE**: `00 ... 00 01` (383 zero bytes followed by `0x01`).

### Compute Cost

`ALT_BN128_PAIRING_MAP` performs the same Miller loop and final exponentiation
as `ALT_BN128_PAIRING` and differs only in what it writes. It SHOULD be charged
using the same cost model (a base cost plus a per-pair cost), with the larger
output accounted for in the memory access cost as for the other
`sol_alt_bn128_group_op` operations.

### Edge Cases

- `input_size == 0`: succeeds and writes the identity element.
- `input_size` not a multiple of 192: error, nothing written.
- Any G1 or G2 point that fails validation: error, nothing written.
- Points at infinity in the input contribute the identity to the product and
  are otherwise valid.
- An unknown `group_op` value, including `7` before feature activation, MUST
  continue to return an error.

### Validator Components Affected

| Validator Component             | Impact                              |
|---------------------------------|-------------------------------------|
| Transaction Execution (Runtime) | New opcode in an existing syscall   |
| Virtual Machine                 | None                                |
| Block Packing                   | None                                |
| Consensus                       | None                                |
| Gossip                          | None                                |
| Turbine                         | None                                |
| Snapshots                       | None                                |
| On-Chain Core BPF Programs      | None                                |

## Alternatives Considered

- **Changing `ALT_BN128_PAIRING`** to return the target group element instead
  of the identity flag. This is a breaking change for every existing caller,
  and the output buffer would grow from 32 to 384 bytes. A new opcode keeps the
  existing behavior intact.
- **Routing the BN254 pairing through `sol_curve_pairing_map`** with a new
  `curve_id`, as done for BLS12-381. The BN254 syscalls already have their own
  entrypoint, input layout, and endianness convention; a second path would
  create two encodings for the same points. A new opcode reuses the existing
  pairing input layout unchanged.
- **A dedicated syscall** for the pairing output. This increases the API
  surface for no functional gain over a new opcode.
- **Adding target group arithmetic** (multiplication, inversion,
  exponentiation) alongside the pairing output. Out of scope here; Fq12
  arithmetic is cheap enough to implement in BPF when needed and can be added
  as separate opcodes later.

## Impact

- Exposing the pairing output enables protocols that consume the target group
  element directly (comparison against precomputed values, multi-transaction
  verification, pairing-based encryption), and brings the BN254 syscalls to
  feature parity with the BLS12-381 pairing syscall.
- No existing opcode, input format, or output format is changed. Programs
  using the existing syscalls are not affected.

## Security Considerations

- `ALT_BN128_PAIRING_MAP` MUST apply exactly the same input validation as
  `ALT_BN128_PAIRING`, including the G2 subgroup check. Skipping it would
  produce values outside the target group and make the output unsafe to use as
  key material.
- The output MUST be the canonical encoding of the fully reduced target groupgit 
  element, so that equal elements are byte-identical. Implementations MUST NOT
  leak internal representation details (e.g. Montgomery form) into the output.
- The new opcode changes the behavior of `sol_alt_bn128_group_op` for a
  previously invalid `group_op` value and therefore MUST be feature-gated. It
  performs no computation beyond what `ALT_BN128_PAIRING` already performs, so
  it adds no new denial-of-service surface when charged with the same cost
  model.

## Backwards Compatibility

None. Only a previously invalid opcode value gains a meaning.
