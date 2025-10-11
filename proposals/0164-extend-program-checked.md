---
simd: '0164'
title: ExtendProgramChecked loader-v3 instruction
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Review
created: 2025-08-05
feature: '2oMRZEDWT2tqtYMofhmmfQ8SsjqUFzT6sYXppQDavxwz'
---

## Summary

Adds `ExtendProgramChecked` which is the same as `ExtendProgram`, but checks
that the authority signed.

## Motivation

Currently anybody can extend (increase the size and funds of) programs, which
they do not control (have no authority over). This has basically no effect on
the programs as many choose to deploy with max length anyway, so tailing zeros
are generally not an issue. Additionally, doing so requires transferring funds,
which people rarely do for no gain. However, for the sake of consistency all
program management instructions should require the authority to sign.

## Alternatives Considered

Ignoring it and waiting for loader-v4 as that does not have this issue.

## New Terminology

None

## Detailed Design

Existing instructions should not be changed (so transaction parsers remain
stable), thus a new instruction needs to be added, similar to `SetAuthority`
and `SetAuthorityChecked`.

Adds a new instruction `ExtendProgramChecked` to loader-v3 which:

- The following instruction accounts:
  - 0. `[writable]` The program data account.
  - 1. `[writable]` The program account.
  - 2. `[signer]` The authority.
  - 3. `[]` System program, optional, used to transfer lamports from the payer
  to the ProgramData account.
  - 4. `[signer]` The payer account, optional, that will pay necessary rent
  exemption costs for the increased storage size.
- The following instruction data:
  - The enum discriminant `8`.
  - `u32` (little endian): Number of bytes to extend the program with.

`ExtendProgramChecked` behaves exactly the same as `ExtendProgram`, except
that:

- It is initially disabled. The associated feature gate must then enable
`ExtendProgramChecked` and disable `ExtendProgram`.
- It is available to be invoked via CPI.
- It expects the instruction account at index 2 to be the authority and
the two optional parameters are shifted up by one each.
- Immediately after the check that the program is not finalized
(which throws `Immutable`) it additionally must check (in that order) that:
  - the provided authority matches the one stored in the program data account,
  otherwise throwing `IncorrectAuthority`
  - the provided authority is a signer, otherwise throwing
  `MissingRequiredSignature`

`InvalidInstructionData` must be thrown when a disabled instruction is invoked.

## Impact

A program is usually extended for a subsequent upgrade / redeployment which
does require an upgrade authority anyway. However, if that upgrade authority
is only available in CPI then a program can only be extended 10 KiB at a time.
If the upgrade authority is avaliable to top level instructions, then no such
limiting factor exists and a program can be extended to full length in one
(top level) instruction.

For programs which do self upgrade (have their own upgrade authority) via a CPI
call, they will have to first upgrade themselves to support the send the
`ExtendProgramChecked` instruction, before the feature of this SIMD becomes
active. We recommend to also add support for the loader-v3 `Migrate` (to
loader-v4) instruction and the new loader-v4 program management instructions
while at it.

For programs which are upgraded by an external upgrade authority via the CLI,
there should be no noticable difference for dApp developers. The CLI will check
the active feature set and either send `ExtendProgram` or
`ExtendProgramChecked` accordingly.

Independently of which upgrade path is used, one can opt to preemptively extend
the program to maximum size before the feature of this SIMD becomes active,
provided they are willing to lock the required rent exemption funds. Loader-v3
does not support truncation / shrinking of programs, but loader-v4 does. Thus,
after a program is migrated to loader-v4 it is possible to undo this and
retrieve the locked funds, sending them to a recipient of your choosing,
provided the programs authority signs.

## Security Considerations

None.
