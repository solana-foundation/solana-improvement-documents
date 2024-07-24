---
simd: 'XXXX'
title: Explicit versioning and feature gating of SBPF programs
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Draft
created: 2024-07-15
---

## Summary

An explicit versioning system for SBPF programs, that allows enabling or disabling features on a per-program basis. 

## Motivation

SBPF has evolved over the years and will continue to evolve. Changes are necessary among other reasons to improve performance, address security concerns and provide new, better features to dapp developers. 

Today, the only way to introduce changes to the program runtime is via feature gates [link that explains what feature gates are]. For example we have used feature gates in the past to restrict new deployments of programs owned by deprecated loaders. Feature gates alone are not sufficient to evolve SBPF though - we are missing a mechanism to enable (or disable) specific features on a per-program basis. 

As an example, over two years ago we decided to change the way the stack works for SBPF programs - from allocating a fixed 4kB per function frame, to letting functions dynamically specify how much stack they need. This change was implemented in both the program runtime and the toolchain (LLVM), but as of today it has not yet been deployed, because it's essentially too hard to do so: upon executing a program, should the program runtime use the old or the new stack allocation strategy?

We propose to introduce an explicit versioning scheme for SBPF programs: programs will encode which SBPF version they are compiled for; based on this version, the program runtime will alter its behavior. 

## New Terminology

SBPF version: the SBPF version number a program was compiled for.

## Detailed Design

Every program must signal which SBPF version it was compiled for via the `e_flags` field in the [ELF header](https://refspecs.linuxfoundation.org/elf/gabi4+/ch4.eheader.html). Thus, the feature gating would not be runtime global, and not be based on the loader, but on every program individually. The `e_flags` field is effectively a toolchain compatibility version number as each change set will be a superset !!!NOT correct, eg dynamic frames is not a superset!!!  of all changes that came before. In order to prevent an "extension hell" it is not possible to opt into specific changes, instead the entire change set is reduced to a single SBPF version number.

Block explorers are recommended to display the SBPF version field for program accounts.

Based on this SBPF version field a validator implementation must load and execute each individual program differently according to the SBPF version it requires. Introduction feature gates (one for each SBPF version) control the allowed re/deployed and execution of programs. Upon activation of an introduction feature gate for a new SBPF version programs compiled for:
- any earlier SBPF version must be rejected by the loader (can not be re/deployed anymore)
- exactly the SBPF version must be accepted by the loader (can now be re/deployed)
- any earlier SBPF version must be executed (can still be executed)
- exactly the SBPF version must be executed (can now be executed)

Deprecation feature gates control program execution and fees thereof (CU costs). Instead of having each version be deprecated individually these would happen in larger blocks composed of multiple version numbers to reduce the amount of redeployment required from dapp developers. In order to avoid a "rugpull" like surprise for users of programs which depend on a versions that are about to be deprecated a linear "ramp up" in CU costs must commence at feature activation and finally end in the denial of execution at a fixed slot offset after the feature activation.

## Alternatives Considered

### Use the runtime global feature gating mechanism
This would mean all programs need to be redeployed simultaneously at feature activation and that they would each require an alternative version to be prepared and uploaded to be switched to (redeployed) on feature activation. This would increase the complexity of program management in loaders and require dapp developers to double the locked tokens in their deployed programs.

It also means that validator implementations would have to handle a massive amount of loading, parsing, relocation, verification and possibly compilation of programs in a single block. It took the Agave team two years to develop, implement, test and deploy this capability. It is desirable for other validator implementations to avoid having to do the same.

### Coupling the feature gating to the loader, not the individual program accounts
The version number could be implied by the loader a program was deployed in, effectively shifting the encoding from the program account content to the owner field. This would require a new loader address for each SBPF version, as the program management and execution logic could be shared for the most part. However, new loaders would necessitate a way to migrate programs between loaders upon redeployment.

## Impact

Active dapp developers who redeploy their programs regularly will (upon feature activation) be forced to also keep their toolchain up-to-date, which they most likely do anyway.

Less active dapp developers who do not redeploy their programs will be nagged by their users to redeploy their programs once the deprecation CU cost ramps up.

Finalized programs which simply can not be redeployed will instead have their user base slowly migrate their funds to alternative programs.

Validator implementations will have to support multiple code paths in the time between the introduction feature activation and the fixed slot offset after the deprecation feature activation.

## Security Considerations

None.

## Drawbacks

Programs will break and if they are finalized their addresses and deployment cost will be burnt.
