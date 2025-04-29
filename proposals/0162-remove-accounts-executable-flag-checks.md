---
simd: '0162'
title: Remove Accounts `is_executable` Flag Checks
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Implemented
created: 2024-07-16
feature: FXs1zh47QbNnhXcnB6YiAQoJ4sGB91tKF3UFHLcKT7PM
extends: 0093
---

## Summary

Remove all checks of accounts `is_executable` flag which can throw errors in
the runtime.

## Motivation

Currently, a program account which satisfies the following conditions can be
invoked / executed:

- Has the `is_executable` flag set
- Is owned by a loader
- Contains a successfully verified ELF, contains the address of a program data
account which does or is a built-in program

The second condition can be used as a performance optimization to filter out
accounts which definitely do not contain programs and the third condition alone
would be sufficient to guarantee correct execution. The first condition is
however is not only useless, it is downright deceptive and entangled with the
confusing proxy account workaround in loader-v3.

Originally, the `is_executable` flag meant that a program was deployed and
finalized, which effectively renders its program account read-only forever,
as the `is_executable` can not be cleared / reset.

With the introduction of the upgradeable loader (loader-v3), instead of
removing the first condition back then, a workaround was created: The program
account became a proxy account which has the `is_executable` flag set and then
points to the actual program data account which does not have the
`is_executable` flag set. Meaning the `is_executable` flag now neither does
reliably indicate that an account contains a program (as the program data
account does not have the flag set), nor does it indicate that a program is
finalized (as the program account has it set while the program remains
upgradeable).

Removing the first condition not only removes some redundant error checks from
the runtime and confusion for future maintainers, but more importantly enables
us to deploy loader-v4 which is upgradeable like loader-v3 but without the need
for proxy accounts.

## New Terminology

None.

## Detailed Design

This proposal aims to unblock loader-v4 with minimal impact on the ecosystem by
only removing checks of the `is_executable` flag. A complete removal of the
flag can be addressed in a subsequent proposal. Thus, the following must remain
unaffected for now:

- Setting of the `is_executable` flag during program deployment in loader-v3
- Calculation of the account hashes
- Minimization of snapshots
- Serialization of instruction accounts `is_executable` flag for dapps
- CPI ignores changes made by the caller to instruction accounts which have
the `is_executable` flag set

These checks of the `is_executable` flag must be removed:

- `ExecutableLamportChange` during execution (transaction succeeds instead)

These checks of the `is_executable` flag do not influence whether transactions
fail or succeed because they are all covered by other checks. Thus, only the
thrown error codes will change, which does not affect consensus. Nevertheless,
they should still be removed because all implementations should aim to produce
the same error codes:

- during transaction loading:
  - `InvalidProgramForExecution` (fallthrough to `UnsupportedProgramId` during
  execution)
  - `AccountNotExecutable` (fallthrough to `UnsupportedProgramId` during
  execution)
  - Meaning the transaction loading checks are effectively deferred until
execution, which gives users more complete transaction logs.

- in the `Upgrade` instruction of loader-v3:
  - `AccountNotExecutable`
  (fallthrough to `IncorrectProgramId` or `InvalidAccountData`, depending on
  the owner)

- during execution:
  - `AccountNotExecutable` (fallthrough to `UnsupportedProgramId` during
  execution)
  - `IncorrectProgramId` (fallthrough to `UnsupportedProgramId` during
  execution)
  - `ExecutableDataModified`
  (fallthrough to `ExternalAccountDataModified` during execution)
  - `ExecutableModified`
  (is unreachable, as it is overshadowed by owner and writability checks)
  - `ModifiedProgramId`
  (is unreachable, as it is overshadowed by owner and writability checks)

Similarly the following checks during execution, unrelated to the
`is_executable` flag, but related to whether an account contains a program or
not, should be changed to throw `UnsupportedProgramId` instead:

- `InvalidAccountData` for programs which are closed, in visibility delay,
failed verification or not owned by a loader
- `IncorrectProgramId` for unrecognized built-in programs

All in all, the following error messages related to invocation of program
accounts will be coalesced into `UnsupportedProgramId`, but some of them will
remain in use in other circumstances unrelated to program execution:

- `InvalidProgramForExecution`
- `AccountNotExecutable`
- `InvalidAccountData`
- `IncorrectProgramId`
- `UnsupportedProgramId`

## Alternatives Considered

None.

## Impact

Error codes of these conditions, which are rarely triggered, will change.

The only consensus relevant change is that it will become possible (for
everybody) to donate funds to program accounts. That however is expected not to
break any existing programs.

## Security Considerations

None.
