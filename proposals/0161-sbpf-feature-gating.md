---
simd: '0161'
title: SBPF versioning and feature gating
authors:
  - Alexander Meißner
  - Alessandro Decina
category: Standard
type: Core
status: Review
created: 2024-07-15
---

## Summary

An explicit versioning system for SBPF programs, which allows enabling or
disabling features on a per-program basis.

## Motivation

SBPF has evolved over the years and will continue to evolve. Changes are
necessary among other reasons to improve performance, address security concerns
and provide new, better features to dapp developers.

Today, the only way to introduce changes to the program runtime is via [feature
gates](
https://github.com/solana-foundation/solana-improvement-documents/issues/76).
For example we have used feature gates in the past to restrict new deployments
of programs owned by deprecated loaders. Feature gates alone are not sufficient
to evolve SBPF though - we are missing a mechanism to enable (or disable)
specific features on a per-program basis.

As an example, over two years ago we decided to change the way the stack works
for SBPF programs - from allocating a fixed 4kB per function frame, to letting
functions dynamically specify how much stack they need. This change was
implemented in both the program runtime and the toolchain (LLVM), but as of
today it has not yet been deployed, because it's essentially too hard to do so:
upon executing a program, should the program runtime use the old or the new
stack allocation strategy?

We propose to introduce an explicit versioning scheme for SBPF programs:
programs will encode which SBPF version they are compiled for; based on this
version, the program runtime will alter its behavior. In order to prevent an
"extension hell" it will not be possible to opt into specific changes via
flags, instead an entire set of changes is reduced to a single SBPF version.

## New Terminology

SBPF version: the SBPF version number a program was compiled for.

## Detailed Design

Every program must signal which SBPF version it was compiled for via
the `e_flags` field in the
[ELF header](https://refspecs.linuxfoundation.org/elf/gabi4+/ch4.eheader.html).
Block explorers are recommended to display this SBPF version field for program
accounts.

For each SBPF version two feature gates will control its availability:

- One to enable deployment and execution of that SBPF version, which includes
disabling deployment (but not execution) of all earlier versions.
- Another to first disincentivize (via CU costs) and later disable execution
of that SBPF version, so that the runtime and virtual machine can reduce their
complexity again. This feature gate can be shared by multiple SBPF versions,
effectively deprecating larger blocks of versions in order to reduce the amount
of redeployment required from dapp developers.

### Version discriminator

Currently the protocol deems every value of `e_flags` which is not `0x0020` as
being SBPF v0 and thus valid. This clearly does not scale for multiple versions
and must therefore be changed. With the activation of the first feature gate
which enables the deployment and execution of the first new SBPF version the
discriminator must switch to directly interpret `e_flags` as the SBPF version.
Meaning a value of `0x0000` is SBPFv0, `0x0001` is SBPFv1, etc.

### Example

| SBPF version | becomes deployable and executable | ceases to be deployable |
| ------------ | --------------------------------- | ----------------------- |
| v0           |                                   | Feature gate A          |
| v1           | Feature gate A                    | Feature gate B          |
| v2           | Feature gate B                    | Feature gate C          |
| v3           | Feature gate C                    | Feature gate D          |
| v4           | Feature gate D                    | Feature gate E          |
| v5           | Feature gate E                    | Feature gate F          |
| v6           | Feature gate F                    |                         |

| SBPF version | ceases to be executable |
| ------------ | ----------------------- |
| v0           | Feature gate G          |
| v1           | Feature gate G          |
| v2           | Feature gate G          |
| v3           | Feature gate H          |
| v4           | Feature gate H          |
| v5           | Feature gate H          |
| v6           |                         |

## Alternatives Considered

### Use the runtime global feature gating mechanism

This would mean all programs need to be redeployed simultaneously at feature
activation and that they would each require an alternative version to be
prepared and uploaded to be switched to (redeployed) on feature activation.
This would increase the complexity of program management in loaders and require
dapp developers to double the locked tokens in their deployed programs.

It also means that validator implementations would have to handle a massive
amount of loading, parsing, relocation, verification and possibly compilation
of programs in a single block. It took the Agave team two years to develop,
implement, test and deploy this capability. It is desirable for other validator
implementations to avoid having to do the same.

### Coupling the feature gating to the loader, not the individual program accounts

The version number could be implied by the loader a program was deployed in,
effectively shifting the encoding from the program account content to the owner
field. This would require a new loader address for each SBPF version, as the
program management and execution logic could be shared for the most part.
However, new loaders would necessitate a way to migrate programs between
loaders upon redeployment.

## Impact

Active dapp developers who redeploy their programs regularly will (upon feature
activation) be forced to also keep their toolchain up-to-date, which they most
likely do anyway.

Less active dapp developers who do not redeploy their programs will be nagged
by their users to redeploy their programs once the deprecation CU cost ramps
up.

Finalized programs that simply can not be redeployed will instead have their
user base slowly migrate their funds to alternative programs.

Validator implementations will have to support multiple code paths in the time
between the introduction feature activation and the fixed slot offset after the
deprecation feature activation.

## Security Considerations

None.

## Drawbacks

Programs will break and if they are finalized their addresses and deployment
cost will be burnt.
