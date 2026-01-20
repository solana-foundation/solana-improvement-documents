---
simd: '0430'
title: Relax Program Buffer Constraints
authors:
    - Dean Little (Blueshift)
    - Joe Caulfield (Anza)
category: Standard
type: Core
status: Review
created: 2025-12-18
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD proposes relaxing current constraints on program buffers used by the
`DeployWithMaxDataLen` (initial deployment) and `Upgrade` (redeployment)
instructions, currently requiring them to:

- Be owned by `BPFLoaderUpgradeab1e11111111111111111111111`, and
- Share an upgrade authority with the program being deployed or upgraded

## Motivation

Currently, it is not feasible to support several advanced use cases for 
program buffers, including:

- Sponsored deployments
- Permissionless buffer reuse
- Retracting erroneous upgrades using a common buffer
- On-chain recompilation

By removing these constraints, the loader can support a broader range of 
advanced and flexible deployment workflows.

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

After the feature is activated, the program will no longer return
`IncorrectAuthority` when the buffer's authority does not match the
program's authority:

- `DeployWithMaxDataLen`: The buffer's authority no longer must match the
  authority that will be set on the deployed program.
- `Upgrade`: The buffer's authority no longer must match the upgrade
  authority stored on the program account.

Note that the authority account must still be provided in the same position
for both instructions and must still sign the transaction. Only the
`IncorrectAuthority` check is removed; the `MissingRequiredSignature` check
remains enforced.

## Alternatives Considered

- Introduce a new loader that enables these behaviors by default
- Only relax the authority constraint (similar outcome, but requires CPI)
- Retain existing constraints and make no changes

## Impact

This change enables more flexible program deployment workflows, unlocking new 
use cases for developers.

## Security Considerations

This proposal introduces two new, strictly opt-in potential attack vectors:

1. In multisig deployment flows (e.g., Squads), if a quorum authorizes an 
upgrade using a buffer it does not own, the buffer could be modified by a 
third party prior to deployment. This introduces additional security 
considerations beyond those enforced by the multisig quorum itself.
2. If a buffer is owned by a third-party program, that program may retain 
write access to the buffer account irrespective of its upgrade authority. 
This creates a potential supply chain attack vector if the security 
assumptions of the owner program are not carefully evaluated.

## Backwards Compatibility

This feature relaxes existing Loader V3 constraints and is therefore not 
backwards compatible for consensus, necessitating a feature gate. For CLI and 
tooling, the change is fully backwards compatible.
