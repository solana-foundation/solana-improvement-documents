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

This SIMD proposes changing the default behavior of program upgrades to
automatically resize the program data account so that its ELF region matches
the length of the ELF being deployed. The total account size remains the
program data metadata header plus the ELF length. If the new ELF is smaller,
surplus lamports are refunded to the spill account. If the new ELF is larger,
the account is extended accordingly.

## Motivation

Currently, Loader v3 program data accounts may be extended via the
`ExtendProgram` instruction, but cannot be retracted. As program sizes decrease
due to SDK improvements such as Pinocchio, this limitation results in program
data accounts remaining larger than necessary, with no mechanism to reclaim the
rent paid for unused bytes. This unnecessarily increases rent costs and program
loading overhead.

Additionally, upgrading a program to a larger ELF requires issuing a separate
`ExtendProgram` instruction prior to `Upgrade`. This additional step increases
operational complexity and has been a recurring point of debate in other
proposals.

## New Terminology

N/A

## Detailed Design

The `Upgrade` instruction will be updated to automatically resize the program
data account so that its ELF region matches the length of the ELF in the
buffer being deployed. The program data account retains its existing metadata
header (`UpgradeableLoaderState::ProgramData`, containing the slot and
upgrade authority), so the resulting account size is
`PROGRAMDATA_METADATA_SIZE + elf_length`. This applies in both directions: the
account may grow or shrink as needed.

**Shrinking:** If the new ELF is smaller than the current program data account's
ELF region, the account will be retracted to the new size. Surplus lamports from
the reduced rent requirement for the program data account will be refunded to
the spill account.

**Growing:** If the new ELF is larger than the current program data account's
ELF region, the account will be extended to accommodate the new ELF. New
lamports for the increased rent requirement are expected to be either credited
to the program data account or available in the buffer account prior to the
execution of the `Upgrade` instruction. If the available lamports are
insufficient to satisfy the new rent requirement, the upgrade will fail.

### Buffer Accounts

Currently, the `Upgrade` instruction will:

- Deallocate the buffer account for garbage collection.
- Apply the buffer's lamports to the program data's new rent requirement, if
  necessary.
- Sweep the remaining buffer lamports to the spill account.

This behavior is unchanged by this proposal. For any proposals that may modify
the behavior of buffer account closure, the same lamport requirements detailed
in the **Growing** paragraph apply.

### Feature Gate

This change will be a feature-gated behavioral change to the existing `Upgrade`
instruction.

## Alternatives Considered

### Shrinking Only

An earlier version of this proposal only supported shrinking, requiring the
`ExtendProgram` instruction to be called before upgrading to a larger ELF. This
approach was rejected in favor of bidirectional resizing to simplify upgrade
workflows and reduce the number of instructions required.

### Separate Lamport Withdrawal

An alternative approach would be to add a new `WithdrawExcessLamports`
instruction, similar to the instruction of the same name in the Token-2022
program. This would allow the program's upgrade authority to claim excess
lamports after the auto-resizing from `Upgrade`. This was rejected in favor of
automatically refunding surplus lamports to the spill account during the
upgrade itself.

## Impact

This proposal results in a lower program footprint in Accounts DB, incentivizes
developers to upgrade to newer, more performant libraries and SDKs, and enables
the recovery of surplus lamports, including those accidentally sent to the
program data address.

## Security Considerations

### CPI Account Growth Limit

When invoking the `Upgrade` instruction via CPI, the 10 KiB per-instruction
account growth limit still applies. If the new ELF requires the program data
account to grow by more than 10 KiB, the upgrade will fail when called via CPI.

Programs requiring larger growth must either:

- Perform the upgrade at the top level of the transaction, or
- Split the growth across multiple instructions using `ExtendProgram` before
  upgrading

## Backwards Compatibility

This change modifies an existing Loader v3 instruction and therefore requires a
feature gate for consensus safety. From an API and tooling perspective, the
change is backwards compatible.
