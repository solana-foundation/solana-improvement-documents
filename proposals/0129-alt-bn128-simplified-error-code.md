---
simd: '0129'
title: Alt_BN128 Syscalls - Simplified Error Code
authors:
  - Emanuele Cesena
category: Standard
type: Core
status: Activated
created: 2024-03-19
feature: [JDn5q3GBeqzvUa7z67BbmVHVdE3EbUAjvFep3weR3jxX](https://github.com/anza-xyz/agave/issues/320)
development:
  - Anza - [Implemented](https://github.com/anza-xyz/agave/pull/294)
  - Firedancer - Implemented
---

## Summary

Simplify the return error code for the family of Alt_BN128 syscalls: 
`sol_alt_bn128_group_op`, `sol_alt_bn128_compression` and `sol_poseidon`.

A single error code is sufficient, in line with all other syscalls.

## Motivation

Syscalls in Solana can return:

- Success, e.g. represented in Rust as `Ok(0)` or `Ok(SUCCESS)`
- Error to the program, e.g. represented in Rust as `Ok(1)`
- Fatal error to the VM, aborting the transaction, e.g. represented in Rust
  as `Err(<something>)`

Most syscalls only have a single error code returned to the program, i.e. `Ok(1)`.
The family of Alt_BN128 syscalls, vice versa, has a richer set of error codes.

This proposal aims to simplify the error value for these syscalls, in line with
all the other syscalls, and simply return `Ok(1)` (in addition to fatal errors,
that are left unchanged).

We stress that multiple error codes cause a maintenance burden for the validators.
Moreover, if two different implementation were to return different error codes,
an attacker could exploit the different behavior to cause consensus failure.

## Alternatives Considered

Leave as is.

## New Terminology

n/a

## Detailed Design

### Syscall `sol_alt_bn128_group_op`

The syscall [`sol_alt_bn128_group_op`](https://github.com/solana-labs/solana/pull/27961)
computes operations on the Alt_BN128 curve, including point addition
in G1, scalar multiplication in G1, and pairing.

The syscall accepts the following inputs:

- `group_op`: the operation to perform:

  - `0`: point add in G1
  - `1`: (reserved for) point sub in G1
  - `2`: scalar multiplication in G1
  - `3`: pairing

- `input`: the serialized inputs to the operation.

Notes:

- Input and output depend on the operation. In all cases they are serialized
  in standard big endian format.
- Points and scalars must be validated.
- Point sub in G1 is not implemented. The `group_op` value `1` is reserved.

**Point add in G1**

Inputs: 2 points in G1.

Output: 1 point in G1.

**Scalar multiplication in G1**

Inputs: 1 point in G1, 1 scalar.

Output: 1 point in G1.

**Pairing**

Inputs: 1 point in G1, 1 point in G2

Output: `1` if the pairing is 1, `0` otherwise.
The output is serialized as a 256-bit big integer.

**Fatal Errors.**

- Validate `group_op` is `0`, `2`, or `3` (known operation).
- Compute units
- Memory mapping for input/output

**Error Code(s).**

Any error caused while computing the operation
is returned as error code `1`, i.e. `Ok(1)` in Rust.

This includes validating that the input has the valid length,
the input points and scalars are valid, and the operation is successful.


### Syscall `sol_alt_bn128_compression`

The syscall [`sol_alt_bn128_compression`](https://github.com/solana-labs/solana/pull/32870)
allows to compress or decompress points in G1 or G2 groups over the Alt_BN128
curve.

The syscall accepts the following inputs:

- `op`: the operation to perform:

  - `0`: G1 compress
  - `1`: G1 decompress
  - `2`: G2 compress
  - `3`: G2 decompress

- `input`: the input point to compress / decompress, serialized in standard
  big endian format.

The output is a point, serialized in standard big endian format.

Note: for performance reasons, this syscall does NOT validate whether the
input is actually an element of G1 or G2.
The intent is to use this syscall in combination with `sol_alt_bn128_group_op`,
that performs validation, and therefore not duplicate the expensive check.

**Fatal Errors.**

- Validate `op` is `0`, `1`, `2`, or `3` (known operation).
- Compute units
- Memory mapping for input/output

**Error Code(s).**

Any error caused while computing the compress or decompress operation
is returned as error code `1`, i.e. `Ok(1)` in Rust.

This includes validating that the input has the valid length,
and decompression is successful.

### Syscall `sol_poseidon`

The syscall [`sol_poseidon`](https://github.com/solana-labs/solana/pull/32680)
computes the Poseidon hash on an array of input values.

The syscall accepts the following inputs:

- `parameters`: `0` to represent the choice of the Alt_BN128 curve.
- `endianness`: `0` for big endian input/output, `1` for little endian.
- `vals_len`: number of input values to hash. Max supported 12 values.
- `vals`: the input values to hash. In line with other hash syscalls,
  this is an array of buffers. For Poseidon, each buffer represents an element
  in the curve scalar field, encoded as 32 bytes, with the specified
  endianness.

The output is an element of the curve scalar field, encoded as 32 bytes,
with the specified endianness.

**Fatal Errors.**

- Validate `parameters` is `0` (known curve).
- Validate `endianness` is `0` or `1` (known endianness).
- Validate `vals_len` is less or equal to 12.
- Compute units
- Memory mapping for input/output

**Error Code(s).**

Any error caused while computing the Poseidon hash function is returned
as error code `1`, i.e. `Ok(1)` in Rust.

This includes validating that the input buffers are valid field elements,
and other errors that may occur while calculating the Poseidon hash.

## Impact

Implementing the error logic inside validators will be much easier and less
error prone.

Dapp developers will have less fine-grained errors, but this is in line with all
the other syscalls.

## Security Considerations

Simplifying to one single error code reduces the risk of two different validator
implementations returining different error codes, which could be exploited
to cause a consensus failure.

This change will also hide some internal implementation details, for example
the err code `TryIntoVecError`, which is a plus from a security perspective.

The implementation should be straightforward: change the return from `Ok(err_num)`
to `Ok(1)`, so low risk.

## Backwards Compatibility

The syscall `sol_alt_bn128_group_op` is enabled in testnet, therefore we'll
feature gate the change. Programs using this syscall may need to adapt to the
simplified error code, but this isn't expected to be an issue in practice.

For simplicity, we'll keep the change to all 3 syscalls under the same
feature gate.
