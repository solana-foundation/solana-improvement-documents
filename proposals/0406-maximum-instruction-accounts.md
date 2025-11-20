---
simd: "0406"
title: Maximum instruction accounts
authors: 
    - Alexander Meißner (Anza)
    - Lucas Steuernagel (Anza)
category: Standard
type: Core
status: Review
created: 2025-11-19
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This SIMD imposes a hard restriction on the maximum number of account 
references that an instruction may declare. There is already a limit of 255 
account references imposed during serialization in ABI v0 and v1, but it does 
not currently apply to builtin invocations or precompiles.

## Motivation

Although there can only be a maximum of 35 account keys in the legacy message 
format and 256 in the v0 format, instructions can have up to 65535 (`u16::MAX`) 
accounts, each being a `u8` index referencing a transaction account.

Allowing instructions to reference more accounts than those contained in the 
transaction is pointless, because it entails some accounts will be aliased, 
and the validator will need to deduplicate them.

Furthermore, since both user deployed programs and CPIs only allow 255 
accounts, such a case can only occur for builtins and precompiles, none of 
which require as many accounts.

## New Terminology

None.

## Detailed Design

For every instruction that goes through program runtime (CPI, user deployed 
program invocation, builtin invocation or precompiles), program runtime must 
check if it references more than 256 accounts, and must throw 
`InstructionError::MaxAccountsExceeded` when that is the case.

## Alternatives Considered

None.

## Impact

Calls to user deployed programs from top level instructions and CPIs will not 
be impacted, since there is already a limit in place.

Instructions that invoke builtin programs or precompiles will error out if 
they have more than 256 accounts.

## Security Considerations

None.
