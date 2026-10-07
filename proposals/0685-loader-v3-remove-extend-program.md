---
simd: '0685'
title: 'Loader V3: Remove ExtendProgram'
authors:
    - Joe Caulfield (Anza)
    - Alexander Meißner (Anza)
    - Dean Little (Blueshift)
category: Standard
type: Core
status: Review
created: 2026-09-25
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD proposes removing the `ExtendProgram` instruction from Loader V3.
With [SIMD-0433], the `Upgrade` instruction resizes the programdata account to
match the ELF being deployed, so extension as a separate step no longer serves
a purpose. After activation, `ExtendProgram` MUST fail.

## Motivation

The `ExtendProgram` instruction exists mainly for two reasons:

- The `Upgrade` instruction could not previously grow the programdata account
- Resizing within CPI is limited to 10 KiB

[SIMD-0433] lifts the first restriction, and a new upgrade workflow for PDA
signers (detailed below) can address the second. This makes `ExtendProgram`
redundant for its original purpose.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0433]: Loader V3: Set Program Data to ELF Length**

    `Upgrade` must be able to grow the programdata account before
    `ExtendProgram` can be removed. Without it, no program could be upgraded to
    a larger ELF.

## New Terminology

N/A

## Detailed Design

The `ExtendProgram` instruction, discriminator `6`, MUST fail with
`InvalidInstructionData`. Its discriminator is retired and MUST NOT be reused.

No other Loader V3 instruction changes. No account layout changes. Programs
already deployed continue to run, upgrade, and close exactly as before.

## Alternatives Considered

### Permissioning `ExtendProgram`

[SIMD-0164] proposed `ExtendProgramChecked`, a new instruction requiring the
upgrade authority's signature. This approach would continue to keep this
redundant step isolated from `Upgrade`, with both held to the same
authority, even after [SIMD-0433] is active.

## Impact

Extension ceases to be a user-facing operation. Deploying a larger program is
a single `Upgrade`, and the rent for the growth is supplied through the buffer
account or credited to the programdata account beforehand, as specified in
[SIMD-0433].

### Program-Owned Upgrade Authorities

Programs with a program-owned upgrade authority that need to grow by more than
10 KiB in one upgrade must implement the authority hand-off described below.

The workflow is a single transaction, signed by the designated "hot" keypair
that serves as the upgrade's top-level signer, with the following
instructions:

1. A CPI from the owning program to Loader V3's `SetAuthority` to assign the
   hot keypair as the new upgrade authority, signed by the PDA authority.
2. A top-level Loader V3 `Upgrade` instruction to perform the upgrade and
   resize the programdata account by more than 10 KiB. This is authorized at
   the top level by the signing hot keypair.
3. A top-level Loader V3 `SetAuthority` instruction to reassign the original
   PDA authority as the upgrade authority, as it was at the beginning.

Programs may also choose to employ one or more "guard" instructions that
assert that the resulting upgrade authority at the end of the transaction is in
fact the correct address.

This new workflow leverages transaction atomicity to perform a safe upgrade
when the programdata account must be resized by more than 10 KiB.

## Security Considerations

The primary security concern is safe implementation of upgrade workflows akin
to the one described in the previous section, for those authorities who truly
require it.

## Backwards Compatibility

This proposal removes an instruction from the Loader V3 interface. It is not
backwards compatible and requires a feature gate for consensus safety.

Any transaction containing `ExtendProgram` fails after activation.

[SIMD-0164]: https://github.com/solana-foundation/solana-improvement-documents/pull/164
[SIMD-0433]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0433-loader-v3-set-program-data-to-elf-length.md
