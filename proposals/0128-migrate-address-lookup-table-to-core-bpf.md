---
simd: '0128'
title: Migrate Address Lookup Table to Core BPF
authors:
  - Joe Caulfield - Anza Technology
category: Standard
type: Core
status: Draft
created: 2024-03-13
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal outlines migrating the builtin Address Lookup Table program to
Core BPF using the procedure outlined in
[SIMD 0088](https://github.com/solana-foundation/solana-improvement-documents/pull/88).

## Motivation

BPF programs offer less complexity than native programs for other clients, such
as Firedancer, since developers will no longer have to keep up with program
changes in their runtime implementations. Instead, the program can just be
updated once.

In this spirit, Address Lookup Table should be migrated to Core BPF.

## Alternatives Considered

The alternative to migrating this program to Core BPF is to keep it as a builtin
program. This would mean each validator client implementation would have to
build and maintain this program with their runtime implementations, including
any future changes to these programs.

## New Terminology

N/A.

## Detailed Design

The program will be migrated to Core BPF using the procedure outlined in
[SIMD 0088](https://github.com/solana-foundation/solana-improvement-documents/pull/88).

The program's upgrade authority will be a multi-sig authority with keyholders
from Anza Technology and may expand to include contributors from other validator
client teams.
In the future, this authority could be replaced by validator governance.

The program will be backwards compatible with its original builtin
implementation, except for the following items:

- The concept of a "recent slot" is now determined by subtracting the slot from
  the clock's current slot, rather than determining if it resides in the
  `SlotHashes` sysvar.

### Changes to Evaluating a Recent Slot

TODO: Information about the recent slot changes and their impact on lookup table
creation and deactivation/closing should be outlined here.

Until the relevant syscall investigation concludes, this will remain in draft.

## Impact

With this change, validator clients would no longer be required to implement and
maintain an Address Lookup Table program built into their runtime. Instead teams
can coalesce to maintain and improve the Core BPF version of Address Lookup
Table each one depends on.

## Security Considerations

The BPF version's upgrade authority will be managed by a multi-sig, and the
security considerations of such an arrangement - including the members of the
authority - should be considered by contributors.

The program's reimplementation poses no new security considerations compared to
the original builtin version.

## Backwards Compatibility

The Core BPF implementation of Address Lookup Table is fully backwards
compatible with its original builtin implementation from an ABI perspective.
However, some behavior has changed.

As outlined above, recent slots are now determined from the clock, so they no
longer consider slots wherein a valid block was produced. Overall this reduces
the cooldown time for a deactivated lookup table.

TODO: What I really want this to say:

The Core BPF implementation of Address Lookup Table will be 100% backwards
compatible with its original builtin implementation.
