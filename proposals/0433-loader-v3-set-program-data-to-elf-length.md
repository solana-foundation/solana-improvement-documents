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
automatically resize the program data account to match the length of the ELF
being deployed. If the new ELF is smaller, surplus lamports are refunded to the
spill account. If the new ELF is larger, the account is extended accordingly.

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

## Dependencies

This proposal depends on the following previously accepted proposal:

- **[SIMD-0430]: Loader V3: Relax Program Buffer Constraints**

    Introduces the `close_buffer` flag for `DeployWithMaxDataLen` and
    `Upgrade` instructions, which determines whether the buffer account is
    closed after the operation

[SIMD-0430]: https://github.com/solana-foundation/solana-improvement-documents/pull/430

## Detailed Design

The `Upgrade` instruction will be updated to automatically resize the program
data account to match the length of the ELF in the buffer being deployed. This
applies in both directions: the account may grow or shrink as needed.

### Shrinking (New ELF is Smaller)

If the new ELF is smaller than the current program data account's ELF region,
the account will be retracted to the new size. Surplus lamports from the reduced
rent requirement will be refunded to the spill account.

### Growing (New ELF is Larger)

If the new ELF is larger than the current program data account's ELF region,
the account will be extended to accommodate the new ELF.

When `close_buffer` is `true`, the buffer account's lamports and the program
data account's existing lamports are combined to meet the new rent-exempt
minimum. If the combined lamports are insufficient, the upgrade will fail with
`InsufficientFunds`.

When `close_buffer` is `false`, the program data account's existing lamports
must already meet the new rent-exempt minimum. If not, the upgrade will fail
with `InsufficientFunds`.

In both cases, any lamports in excess of the rent-exempt minimum are refunded
to the spill account.

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
