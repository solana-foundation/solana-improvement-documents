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

which together form SBPFv3. Thus, there will be a new valid deployment target
before we deactivate the older ones.

## New Terminology

None.

## Detailed Design

After the activation of the associated feature key a validator must fail to
(re-)deploy or finalize programs with any SBPF version older than SBPFv3.

Currently loader-v3 is the only loader with program management enabled, thus
this means for each of the following loader-v3 instructions:

- deployment: `DeployWithMaxDataLen`
- redeployment: `Upgrade`
- finalization: `SetAuthority` and `SetAuthorityChecked` with no new authority

must return `InstructionError::InvalidAccountData` when the ELF contains
a SBPFv0, SBPFv1 or SBPFv2 program. While `InitializeBuffer`, `Write`,
`ExtendProgram` and `Close` remain unaffected.

In order not to interfere with other SIMDs, all core program migrations
(SIMD-0418) and upgrades (SIMD-0495) are exempt from this. These core programs,
while owned by loader-v3, are not using the program management instructions of
loader-v3 anyway.

## Alternatives Considered

Not deprecating some or all older SBPF versions.

## Impact

dApp developers using outdated toolchains will have to update them and adjust
their programs before they can re-/deploy or finalize them.

Furthermore, testing frameworks and mock ups will have to be adapted to either
deactivate this feature or bypass the entire deployment in order to continue
to test older SBPF versions.

### Currently Deployed SBPF Versions per Loader (Mainnet)

Analysis performed using Blueshift's [program-sync] tool.

| Loader                 | SBPFv0 | SBPFv1 | SBPFv2 |
| ---------------------- | ------ | ------ | ------ |
| Loader-v1 (Finalized)  | 136    | 0      | 0      |
| Loader-v2 (Finalized)  | 315    | 0      | 0      |
| Loader-v3 (Finalized)  | 422    | 0      | 0      |
| Loader-v3 (Upgradable) | 17279  | 12     | 41     |

[program-sync]: https://github.com/blueshift-gg/program-sync

## Security Considerations

None.

## Backwards Compatibility

Execution of SBPFv0, SBPFv1 and SBPFv2 will remain supported for now.
