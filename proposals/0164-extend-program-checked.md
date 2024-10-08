---
simd: '0164'
title: ExtendProgramChecked loader-v3 instruction
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Draft
created: 2025-08-05
feature: '97QCmR4QtfeQsAti9srfHFk5uMRFP95CvXG8EGr615HM'
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

Almost none, the CLI will send `ExtendProgramChecked` instead of
`ExtendProgram` once the feature is active. Thus dapp developers should not
notice any difference.

## Security Considerations

None.
