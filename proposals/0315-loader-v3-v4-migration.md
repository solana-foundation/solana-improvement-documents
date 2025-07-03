---
simd: '0315'
title: Loader-v3 to loader-v4 migration
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Review
created: 2024-08-15
feature: 2aQJYqER2aKyb3cZw22v4SL2xMX7vwXBRWfvS4pTrtED
extends: SIMD-0167
---

## Summary

Migration of loader-v3 programs to loader-v4.

## Motivation

In order to remove the issues of loader-v3 (mentioned in SIMD-0167) from
validator implementations, all remaining loader-v3 must be migrated to
loader-v4.

## Alternatives Considered

### Loader-v1 and loader-v2 programs

The two loader-v3 accounts per program in their sum are always larger than
the one resulting loader-v4 account. Thus there is no need for additional
funding. This would not be the case when migrating from older loaders. Meaning
that expanding this SIMD to cover these as well would require a funding source.

### Global Migration: Coordinated in valiator or out of validator

The global migration could be implemented in the validator, however:

- If the global migration mechanism is inside the validator, the risk of it
being detrimental to block production outweights any possible benefits.
- It would have to be coordinated across all validator implementations,
tested, fuzzed, etc. simply a whole lot more work for something which is only
used once.
- It being triggered manually per program or once (via a feature gate) for all
programs changes nothing about it being controlled by a single key.
- The only difference is in having more fine granular control over the
timing in when a specific programs migration is triggered.
- Doing it outside of the validator allows for the process to be aborted or
patched quickly in case things start going sideways.

## New Terminology

None.

## Detailed Design

The feature gate must:

- enable loader-v4 `LoaderV411111111111111111111111111111111111` program
management and execution (see SIMD-0167).
- enable the loader-v3 `BPFLoaderUpgradeab1e11111111111111111111111`
instruction `UpgradeableLoaderInstruction::Migrate`.

### Loader-v3 Instruction: Migrate

- Instruction accounts:
  - `[writable]` The program data account.
  - `[writable]` The program account.
  - `[signer]` The migration authority.
- Instruction data:
  - Enum variant `8u32`
- Behavior:
  - Check that there are at least three instruction accounts,
  otherwise throw `NotEnoughAccountKeys`
  - Check that the program data account is writable,
  otherwise throw `InvalidArgument`
  - Check that the last modified slot (stored in the program data accounts
  header) is less than the current slot if the program data has the state
  `ProgramData`,
  otherwise throw `InvalidArgument`
  - Check that the provided authority is either:
    - the migration authority
    (pubkey is `3Scf35jMNk2xXBD6areNjgMtXgp5ZspDhms8vdcbzC42`)
    - or the upgrade authority stored in the program data account
    - or the program signer if the program is finalized, closed or
    uninitialized
  otherwise throw `IncorrectAuthority`
  - Check that the provided authority is a signer,
  otherwise throw `MissingRequiredSignature`
  - Check that the program account is writable,
  otherwise throw `InvalidArgument`
  - Check that the program account is owned by loader-v3,
  otherwise throw `IncorrectProgramId`
  - Check that the program account has the state `Program`,
  otherwise throw `InvalidAccountData`
  - Check that the program account points to the program data account,
  otherwise throw `InvalidArgument`
  - Clear the program accounts payload data (setting its size to zero)
  - Transfer all funds from the program data account to the program account
  - Assign ownership of the program account to loader-v4
  - If the program data account was not closed / empty or uninitialized:
    - CPI loader-v4 `SetProgramLength` the program account to the program data
    account size minus the loader-v3 header size (45 bytes) and use the
    provided authority.
    - CPI loader-v4 `Copy` the program data account into the program account
    - CPI loader-v4 `Deploy` the program account
    - If the program data account was finalized (upgrade authority is `None`):
      - CPI loader-v4 `Finalize` without a next version forwarding
    - otherwise, if the program data account was not finalized and the
    migration authority (as opposed to the upgrade authority) was provided:
      - CPI loader-v4 `TransferAuthority` to the upgrade authority
  - Clear the program data accounts payload data (setting its size to zero)

## Impact

This changes enables the migration of programs from loader-v3 to loader-v4
without changing their program address via a new loader-v3 instruction. This
will count as a redeployment and thus render the program unavailable for the
rest of the slot (delay visibility).

Once new programs can not be deployed on loader-v3 anymore, the list of all
loader-v3 programs becomes fixed and can be extracted from a snapshot. Using
the added loader-v3 migration instruction and the global migration authority,
the core protocol developers will then migrate all loader-v3 programs to
loader-v4 programs, which once completed:

- removes the need to copy ELFs during program loading to align them.
- allows transaction account loading to be simplified, because every program
would load exactly one account, no need to load the proxy account to get to
the actual program data (which is not listed in the transaction accounts).
- allows the removal of the write lock demotion exception if loader-v3 is
present in a transaction.
- allows dApp devs to resuscitate closed loader-v3 programs if they still
control the program authority. This allows redeployment at the same address
or completely closing the program account in order to retrieve the locked
funds.

## Security Considerations

None.

## Backwards Compatibility

None.
