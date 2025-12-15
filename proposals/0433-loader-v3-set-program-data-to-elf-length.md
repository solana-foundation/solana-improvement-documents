---
simd: '0433'
title: 'Loader V3: Set Program Data to ELF Length'
authors:
    - Joe Caulfield (Anza)
    - Dean Little (Blueshift)
category: Standard
type: Core
status: Review
created: 2025-12-14
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD proposes changing the default behavior of program upgrades to resize
the program data account to the length of the ELF being deployed, refunding any
surplus lamports to a spill account.

## Motivation

Currently, Loader v3 program data accounts may be extended but cannot be
retracted. As program sizes decrease due to SDK improvements such as Pinocchio,
this limitation results in program data accounts remaining larger than
necessary, with no mechanism to reclaim the rent paid for unused bytes. This
unnecessarily increases rent costs and program loading overhead.

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

The `Upgrade` instruction will be updated to automatically resize the program
data account to match the length of the ELF in the buffer being deployed.

If the new ELF is larger than the current program data account, the upgrade will
fail. The account must first be extended to the required size via the
`ExtendProgram` instruction.

If the new ELF is smaller than the current program data account, the account
will be retracted and surplus lamports will be refunded to the spill account.

This change will be a feature-gated behavioral change to the existing `Upgrade`
instruction.

## Alternatives Considered

An alternative approach would be to add a new `WithdrawExcessLamports`
instruction, similar to the instruction of the same name in the Token-2022
program. This would allow the program's upgrade authority to claim excess
lamports after the auto-resizing from `Upgrade`.

## Impact

This proposal results in a lower program footprint in Accounts DB, incentivizes
developers to upgrade to newer, more performant libraries and SDKs, and enables
the recovery of surplus lamports, including those accidentally sent to the
program data address.

## Security Considerations

N/A

## Backwards Compatibility

This change modifies an existing Loader v3 instruction and therefore requires a
feature gate for consensus safety. From an API and tooling perspective, the
change is backwards compatible.
