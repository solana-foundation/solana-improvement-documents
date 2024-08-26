---
simd: "0137"
title: EC Syscalls - Abort on Unsupported Curve/Ops
authors:
  - Sam Kim
category: Standard
type: Core
status: Accepted
created: 2024-04-08
feature: [FuS3FPfJDKSNot99ECLXtp3rueq36hMNStJkPJwWodLh](https://github.com/anza-xyz/agave/issues/1132)
development:
  - Anza - [Implemented](https://github.com/anza-xyz/agave/pull/412)
  - Firedancer - WIP
---

## Summary

Update the curve25519 syscalls functions from returning an error code 1
(`Ok(1)` in rust) to returning an error type
`Err(SyscallError::InvalidAttribute)` when invoked on unsupported curve id or
group op id.

The curve25519 syscall functions should never be invoked on an unsupported
curve or operations and such behavior is an unrecoverable program error.
Updating the syscall functions to return an error type variant will immediately
abort a transaction execution.

## Motivation

There is currently inconsistency between the curve25519 and alt_bn128 syscall
functions on unsupported curve inputs. If the curve25519 syscalls are invoked on
an unsupported curve or group operation id, the curve25519 syscalls return
error code of `1` (`Ok(1)` in rust) while the alt_bn128 syscalls immediately
aborts with error code `SyscallError::InvalidAttribute`
(`Err(SyscallError::InvalidAttribute`). It would be great to update these
functions so that this behavior is handled consistently between the curve
syscalls.

Since calling these syscalls on an unsupported curve or group op should never be
done by a program and is an unrecoverable error, it makes sense to update the
curve25519 syscalls to also immediately abort when invoked on an unsupported
curve id or ops.

## Alternatives Considered

Leave as is.

## New Terminology

N/A

## Detailed Design

The curve25519 syscall functions are organized as follows:

- Curve point validation: on input a curve id and a byte representation of a
  curve point, verifies whether the input represents a valid curve point.
- Curve point group operations: on input a curve id, operation id, and two byte
  representations of curve points, either adds, subtracts, or multiplies the
  input points.
- Curve point multiscalar multiplications: on input a curve id, a vector of
  scalar, and a vector of curve points, performs a multiscalar multiplication on
  the input scalars and points.

Currently, these syscalls accept two possible curve ids: `CURVE25519_EDWARDS=0`
and `CURVE25519_RISTRETTO=1`. If any other curve variant is provided as input,
then the function returns with error code 1.

Similarly, the curve point group operations accept the operation ids `ADD=0`,
`SUB=1`, and `MUL=2`. If any other operation id is provided as input, then the
function returns with error code 1.

The proposed change in this SIMD is to update these syscalls so that on curve
ids that are not `CURVE25519_EDWARDS` or `CURVE25519_RISTRETTO`, and operation
ids that are not `ADD=0`, `SUB=1`, or `MUL=2`, instead of returning with error
code 1, immediately abort the execution, returning
`Err(SyscallError::InvalidAttribute)` as is done in the alt_bn128 syscall
functions.

## Impact

Returning an error `Err(...)` on an unsupported curve or operation will prevent
a program from mishandling an accidental mis-use of the syscall functions. It
will also make the curve25519 and alt_bn128 syscalls for consistent in the way
they handle unsupported curves.

## Security Considerations

A program should never invoke the syscalls on an unsupported curve or operation.
If the syscall function returns `Ok(1)` as is currently implemented, it is still
possible for the program to handle the error and abort. However, if the syscall
function returns an actual error `Err(...)`, then the program will automatically
abort execution, removing the possibility that the caller program handles the
error incorrectly.

## Drawbacks _(Optional)_

None other than potentially delaying the activation of the syscall.

## Backwards Compatibility _(Optional)_

The curve25519 syscalls are enabled on testnet and devnet. Therefore, we will
need to feature gate the change. Programs using this syscall may need to adapt
to the new error handling behavior, but this should not be a big issue since
programs should not generally be calling the syscalls on an unsupported curve to
begin with.
