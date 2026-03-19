---
simd: '0500'
title: Disable deployment of SBPF-v0, -v1 and -v2
authors:
  - Alexander Meißner (Anza)
category: Standard
type: Core
status: Idea
created: 2026-03-17
feature: TBD
supersedes: 0161
---

## Summary

Disable deployment of SBPFv0, SBPFv1 and SBPFv2.

## Motivation

The number of programs using these older SBPF versions will decrease over
time as dApp developers will have to target at least SBPFv3 for re-deployments.
This will eventually allow us to deprecate the execution of these SBPF versions
after their existence (and thus also usage) drops to near zero.

## Dependencies

This proposal depends on the following previously accepted proposals:

- [SIMD-0178](https://github.com/solana-foundation/solana-improvement-documents/pull/178)
- [SIMD-0189](https://github.com/solana-foundation/solana-improvement-documents/pull/189)
- [SIMD-0377](https://github.com/solana-foundation/solana-improvement-documents/pull/377)

which together form SBPFv3. There should be a new valid deployment target
before we deactivate the older ones.

## New Terminology

None.

## Detailed Design

After the activation of the associated feature key a validator must fail to
execute programs with any SBPF version other than v3, throwing the error
message `InstructionError::InvalidAccountData`.

## Alternatives Considered

None.

## Impact

dApp developers using outdated toolchains will have to update them and adjust
their programs before they can re-/deploy them.

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Backwards Compatibility

Execution of SBPFv0, SBPFv1 and SBPFv2 will remain supported for now.
