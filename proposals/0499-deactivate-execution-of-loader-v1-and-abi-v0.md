---
simd: '0499'
title: Deactivate execution of loader-v1 and ABI-v0
authors:
  - Alexander Meißner (Anza)
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Review
created: 2026-03-16
feature: TBD
---

## Summary

Deactivate execution of loader-v1 and ABI-v0.

## Motivation

The trouble with ABIv0 is that it had no alignment padding in its serialization
format and simply ignored any alignment requirements for syscall parameters.
This has since become undefined behavior in Rust. Disabling execution of
loader-v1 would allow us to remove ABIv0 entirely (as it is the only loader
which supports that ABI version) and would reduce the maintenance and auditing
burden in most syscalls significantly.

## New Terminology

None.

## Detailed Design

After the activation of the associated feature key a validator must fail to
execute programs owned by loader-v1 throwing the error message:

- `TransactionError::InvalidProgramForExecution` for top level instructions at
transaction loading time (before executing the first instruction)
- `InstructionError::UnsupportedProgramId` for CPI calls

## Alternatives Considered

Continuing to support this barely used functionality.

## Impact

All programs owned by loader-v1 would stop working forever with their locked
funds effectively burned. This might be relevant for sleeper programs which
have not seen any activity in years.

## Security Considerations

None.
