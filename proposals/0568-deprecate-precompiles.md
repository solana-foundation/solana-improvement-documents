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
and their inter-instruction data referencing is hacky.

## Dependencies

This proposal depends on the following proposals:

- **[SIMD-0565]: Secp256r1 curve syscalls**

    To provide an efficient alternative to the secp256r1 precompile.

[SIMD-0565]: https://github.com/solana-foundation/solana-improvement-documents/pull/565

## New Terminology

None.

## Detailed Design

The addresses of the precompiles are:

- secp256k1: `KeccakSecp256k11111111111111111111111111111`
- secp256r1: `Secp256r1SigVerify1111111111111111111111111`
- ed25519: `Ed25519SigVerify111111111111111111111111111`

With the activation of the feature gate their accounts must be deleted.

After the activation of the feature gate:

- Fee calculation must ignore them, thus not charge `lamports_per_signature`
for them.
- The cost tracker and other cost estimation heuristics should ignore them.
- The callee CPI restrictions for precompiles which throw
`CpiError::ProgramNotSupported` must be removed.

### Validator Components Affected

These changes are limited to the program runtime component
(Transaction Execution).

## Alternatives Considered

### Migrating them to core programs on-chain

Precompiles can use top-level instruction introspection, which is not available
to programs on-chain without the Instructions sysvar. Thus it would require
either breaking changes to the precompile or adding features to the SVM:

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

With the cleanup, after the feature gate is activated on MNB, precompiles
should also be removed from:

- test validator genesis
  - the program accounts of the precompiles
- ledger tool:
  - `fix_testnet_ed25519_precompile_account`
- solana-sdk:
  - `SanitizedMessage`
  - `SVMStaticMessage`
  - `TransactionSignatureDetails`
  - `solana_sdk_ids`
  - `PrecompileError`
- tests, mockups, examples, documentation

## Security Considerations

Implementation-specific guidance will be required for transaction builder
clients and dApp devs who develop the on-chain replacements.

On the validator implementation side this change is a pure removal of features
and should thus not have any security pitfalls.

## Backwards Compatibility

Breaking change: All precompiles will become unavailable.

## Conformance

The change will be tested by 18 transactions, each transaction calling one
of the three precompiles before and after the feature activation, at top level
with a valid signature, at top level with an valid signature and in CPI with a
valid signature.

The expected results **before** the feature activation are:

- at top level with a valid signature: success
- at top level with an invalid signature: `PrecompileError::InvalidSignature`
- in CPI with a valid signature: `CpiError::ProgramNotSupported`

The expected results **after** the feature activation are:

- at top level with a valid signature:
`TransactionError::InvalidProgramForExecution`
- at top level with an invalid signature:
`InstructionError::UnsupportedProgramId`
- in CPI with a valid signature: `InstructionError::UnsupportedProgramId`

[to be filled in before merging]
