---
simd: '0568'
title: Deprecate Precompiles
authors:
  - Alexander Meißner (Anza)
category: Standard
type: Core
status: Review
created: 2026-06-17
feature: [to be filled in before merging]
---

## Summary

Remove support for the secp256k1, secp256r1 and ed25519 precompiles.

## Motivation

Precompiles are rigid, require special handling in validator implementations
and their cross instruction referencing (in ed25519) is hacky.

In a discussion on Discord [^1] consensus was reached to proceed with option
4 "Drop protocol-owned verifiers entirely and let normal programs handle this".

[^1]: https://discord.com/channels/428295358100013066/1164657208613683280/1516459607726358672

## New Terminology

None.

## Detailed Design

After the activation of the feature gate every instruction attempting to invoke
any of the following programs must fail with
`InstructionError::UnsupportedProgramId`:

- secp256k1: `KeccakSecp256k11111111111111111111111111111`
- secp256r1: `Secp256r1SigVerify1111111111111111111111111`
- ed25519: `Ed25519SigVerify111111111111111111111111111`

As a consequence the check in CPI which prevents precompiles from being called
and throws `CpiError::ProgramNotSupported` becomes pointless and must be
skipped as well.

### Edge Cases

The three program accounts will continue to exist and be owned by the native
loader (`NativeLoader1111111111111111111111111111111`) thus they will continue
to count as programs during transaction loading and only be filtered out during
instruction execution.

### Validator Components Affected

These changes are limited to the program runtime component
(Transaction Execution).

## Alternatives Considered

### Migrating them to core programs on-chain

The ed25519 one requires top-level instruction introspection, which is not
available to programs on-chain out of the box right now. Thus it would require
either breaking changes to the precompile or adding features to the SVM:

- Require users to pas in all bytes to verify in the precompile instruction,
forbidding cross referencing other instructions.
- Require users to pass in the instructions sysvar account.
- Add an instructions sysvar syscall, or some other mechanism to access it.

### Where to place the check

The check could also be placed at transaction loading time and throw
`TransactionError::InvalidProgramForExecution` when any top-level instruction
for a precompile is detected. This however would still leave the CPI callee
special case relevant.

## Impact

The community will have to develop their own on-chain replacements.
However, this is also an opportunity as solutions using crypto syscalls can be
cheaper compared to the lamports charged for signature verification at the
transaction level.

According to Sam Kim about 44% of ed25519 invocations access sibling top-level
instruction data. Meaning these would not be possible to fill in with an
exact replacement right now
(see [alternatives considered](#migrating-them-to-core-programs-on-chain)).

## Security Considerations

Implementation-specific guidance will be required for transaction builder
clients and dApp devs who develop the on-chain replacements. Preferably we
should provide canonical replacements, which is different from a core program
as these replacements would life under different addresses and have different
interfaces.

On the validator implementation side this change is a pure removal of features
and should thus not have any security pitfalls.

## Backwards Compatibility

Breaking change: All precompiles will become unavailable.

## Conformance

The change will be tested by three transactions, each transaction calling one
of the three precompiles before and after the feature activation.

[to be filled in before merging]
