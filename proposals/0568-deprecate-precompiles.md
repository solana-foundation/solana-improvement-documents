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

## New Terminology

None.

## Detailed Design

The addresses of the precompiles are:

- secp256k1: `KeccakSecp256k11111111111111111111111111111`
- secp256r1: `Secp256r1SigVerify1111111111111111111111111`
- ed25519: `Ed25519SigVerify111111111111111111111111111`

With the activation of the feature gate their accounts must be deleted.

After the activation of the feature gate:

- Every transaction invoking them with top-level instructions must
throw `TransactionError::InvalidProgramForExecution` during transaction
loading time.
- Every instruction invoking them with in CPI must
throw `InstructionError::UnsupportedProgramId` instead of
`CpiError::ProgramNotSupported` as they currently do.

### Validator Components Affected

These changes are limited to the program runtime component
(Transaction Execution).

## Alternatives Considered

### Migrating them to core programs on-chain

The ed25519 one requires top-level instruction introspection, which is not
available to programs on-chain without the Instructions sysvar. Thus it would
require either breaking changes to the precompile or adding features to the
SVM:

- Require users to pass in all bytes to verify in the precompile instruction,
forbidding cross referencing other instructions.
- Require users to pass in the instructions sysvar account.
- Add an instructions sysvar syscall, or some other mechanism to access it.

## Impact

The community will have to develop their own on-chain replacements. However,
this is also an opportunity as solutions using crypto syscalls can be cheaper
compared to the lamports charged for signature verification at the transaction
level.

44% of ed25519 invocations access sibling top-level instruction data. Meaning
these would not be possible to fill in with an exact replacement right now
(see [alternatives considered](#migrating-them-to-core-programs-on-chain)).

## Security Considerations

Implementation-specific guidance will be required for transaction builder
clients and dApp devs who develop the on-chain replacements.

On the validator implementation side this change is a pure removal of features
and should thus not have any security pitfalls.

## Backwards Compatibility

Breaking change: All precompiles will become unavailable.

## Conformance

The change will be tested by three transactions, each transaction calling one
of the three precompiles before and after the feature activation.

[to be filled in before merging]
