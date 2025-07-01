---
simd: '0167'
title: Loader-v4
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Review
created: 2024-08-15
feature: 2aQJYqER2aKyb3cZw22v4SL2xMX7vwXBRWfvS4pTrtED
---

## Summary

A new upgradeable loader which only requires a single account per program.

## Motivation

Loader-v3, which is currently the only deployable loader, requires two accounts
per program. And has three account types: Program, program-data and buffer.
This was a workaround to circumvent the finality of the `is_executable` flag,
which will be ignored by the program runtime from SIMD-0162 onwards.
Consequentially, this setup of the program account being a proxy account,
containing the address of the actual program data account, is no longer
necessary and should be removed. Likewise the distinction of executable program
accounts and non-executable buffer accounts is also no longer necessary and
should be removed as well.

In loader-v3 every instruction which modified the program data had to re-verify
the ELF in the end. Instead we are now aiming for a more modular workflow which
explicates these steps in a Retract-Modify-Redeploy sequence of instructions
that allows multiple modifications to share one verification of the ELF in the
end. This sequence can be a single transaction or split across multiple
transactions.

Another issue with loader-v3 is that the executable file stored in the
programdata account is misaligned relative to the beginning of the account.
This currently requires a copy in the ELF loader to re-align the program.

Additionally, there currently is no complete specification of the loaders
program management instructions. This proposal would thus fill that gap once
loader-v4 goes into production.

See impact for further motivation.

## Alternatives Considered

A delay-visibility-free redeployment could be achieved by keeping the swap
program around until the end of the slot. This would however mean that two
accounts per program must be loaded until the dapp developer reclaims the
second one. That would defy the purpose of this proposal which is to get rid
of the proxy account.

## New Terminology

None.

## Detailed Design

The feature gate must:

- enable loader-v4 `LoaderV411111111111111111111111111111111111` program
management and execution.
- enable the loader-v3 `BPFLoaderUpgradeab1e11111111111111111111111`
instruction `UpgradeableLoaderInstruction::Migrate` (see SIMD-0315).

### Owned Program Accounts

Accounts of programs owned by loader-v4 must have the following layout:

- Header (which is 48 bytes long):
  - `u64` Slot in which the program was last deployed, retracted or
  initialized.
  - `[u8; 32]` Authority address which can send program management
  instructions. Or if the status is finalized, then the address of the next
  version of the program.
  - `u64` status enum:
    - Enum variant `0u64`: Retracted, program is in maintenance
    - Enum variant `1u64`: Deployed, program is ready to be executed
    - Enum variant `2u64`: Finalized, same as `Deployed`, but can not be
    modified anymore
- Body:
  - `[u8]` The programs executable file

Verification the program account checks in the following order that:

- the owner of the program account is loader-v4,
otherwise throw `InvalidAccountOwner`
- the program account is at least as long enough for the header,
otherwise throw `AccountDataTooSmall`
- the program account is writable, otherwise throw `InvalidArgument`
- the provided authority (instruction account at index 1) signed,
otherwise throw `MissingRequiredSignature`
- the authority stored in the program account is the one provided,
otherwise throw `IncorrectAuthority`
- the status stored in the program account is not finalized,
otherwise throw `Immutable`

### Execution / Invocation

Invoking programs owned by loader-v4 checks in the following order that:

- the owner of the program account is loader-v4
- the program account is at least as long enough for the header
- the status stored in the program account is not retracted
- the program account was not deployed within the current slot (delay
visibility)
- the executable file stored in the program account passes executable
verification

failing any of the above checks must throw `UnsupportedProgramId`.

### Program Management Instructions

All program management instructions must cost 2000 CUs.

#### Write

- Instruction accounts:
  - `[writable]` The program account to write to.
  - `[signer]` The authority of the program.
- Instruction data:
  - Enum variant `0u32`
  - `u32` Byte offset at which to write the given bytes
  - `[u8]` Chunk of the programs executable file
- Behavior:
  - Check there are at least two instruction accounts,
  otherwise throw `NotEnoughAccountKeys`
  - Verify the program account
  - Check the status stored in the program account is retracted,
  otherwise throw `InvalidArgument`
  - Check that the end offset (sum of offset and length of the chunk) does
  not exceed the maximum (program account length minus the header size),
  otherwise throw `AccountDataTooSmall`
  - Copy the chunk into the program account at the offset shifted by the
  header size

#### Copy

- Instruction accounts:
  - `[writable]` The program account to copy to.
  - `[signer]` The authority of the program.
  - `[]` The program(data) account to copy from.
- Instruction data:
  - Enum variant `1u32`
  - `u32` Byte offset at which to write
  - `u32` Byte offset at which to read
  - `u32` Length of the chunk to copy in bytes
- Behavior:
  - Check there are at least three instruction accounts,
  otherwise throw `NotEnoughAccountKeys`
  - Check that program account and source account do not alias,
  otherwise throw `AccountBorrowFailed`
  - Verify the program account
  - Check the status stored in the program account is retracted,
  otherwise throw `InvalidArgument`
  - Check that the source account is owned by loader v1, v2, v3 or v4,
  otherwise throw `InvalidArgument`
  - and look-up the source header size:
    - loader-v1: 0 bytes
    - loader-v2: 0 bytes
    - loader-v3: 45 bytes
    - loader-v4: 48 bytes
  - Check that the source end offset (sum of source offset and length) does
  not exceed the maximum (source account length minus the source header size),
  otherwise throw `AccountDataTooSmall`
  - Check that the destination end offset (sum of destination offset and
  length) does not exceed the maximum (program account length minus the loader-v4
  header size), otherwise throw `AccountDataTooSmall`
  - Copy the chunk between the program accounts at the offsets, each shifted by
  the header size of their loader (account owner) respectively

#### SetProgramLength

- Instruction accounts:
  - `[writable]` The program account to change the size of.
  - `[signer]` The authority of the program.
  - `[writable]` Optional, the recipient account.
- Instruction data:
  - Enum variant `2u32`
  - `u32` The new size after the operation.
- Behavior:
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - If this is an initialization (program account length is too short to
  contain the header):
    - the owner of the program account is loader-v4,
    otherwise throw `InvalidAccountOwner`
    - the program account is writable, otherwise throw `InvalidArgument`
    - the provided authority (instruction account at index 1) signed,
    otherwise throw `MissingRequiredSignature`
  - If this is not an initialization:
    - Verify the program account
    - Check that the status stored in the program account is retracted,
    otherwise throw `InvalidArgument`
  - Check that there are enough funds in the program account for rent
  exemption, otherwise throw `InsufficientFunds`
  - If there are more than enough funds:
    - Check there are at least three instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
    - Check that the recipient account (instruction account at index 2) is
    writable, otherwise throw `InvalidArgument`
    - If a recipient account was provided that is not the program account:
      - Transfer the surplus from the program account to the recipient account
    - otherwise, if the requested new size is zero throw `InvalidArgument`
  - If the requested new size is zero:
    - Delete the entire program account, including the header
  - If the requested new size is greater than zero:
    - Set the length of the program account to the requested new size plus
    the header size
    - In case that this is an initialization, also initialize the header:
      - Set the `is_executable` flag to `true`
      - Set the slot to zero, **not** the current slot
      - Set the authority address (from the instruction account at index 1)
      - Set the status to retracted

#### Deploy

- Instruction accounts:
  - `[writable]` The program account to deploy.
  - `[signer]` The authority of the program.
- Instruction data:
  - Enum variant `3u32`
- Behavior:
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account
  - Check that the slot stored in the program account is not the current
  (deployment cooldown), otherwise throw `InvalidArgument`
  - Note: The cooldown enforces that each pair of an address and a slot can
  uniquely identify a deployment of a program, which simplifies caching logic.
  - Check that the status stored in the program account is retracted
    otherwise throw `InvalidArgument`
  - Check that the executable file stored in the program account passes
  executable verification
  - Change the slot in the program account to the current slot
  - Change the status stored in the program account to deployed

#### Retract

- Instruction accounts:
  - `[writable]` The program account to retract.
  - `[signer]` The authority of the program.
- Instruction data:
  - Enum variant `4u32`
- Behavior:
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account
  - Check that the slot stored in the program account is not the current
  (deployment cooldown), otherwise throw `InvalidArgument`
  - Check that the status stored in the program account is deployed,
    otherwise throw `InvalidArgument`
  - Note: The slot is **not** set to the current slot to allow a
  retract-modify-redeploy-sequence within the same slot or even within the
  same transaction.
  - Change the status stored in the program account to retracted

#### TransferAuthority

- Instruction accounts:
  - `[writable]` The program account to change the authority of.
  - `[signer]` The current authority of the program.
  - `[signer]` The new authority of the program.
- Instruction data:
  - Enum variant `5u32`
- Behavior:
  - Check there are at least three instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account
  - Check that the new authority (instruction account at index 2)
  signed as well, otherwise throw `MissingRequiredSignature`
  - Check that the authority stored in the program account is different
  from the one provided, otherwise throw `InvalidArgument`
  - Copy the new authority address into the program account

#### Finalize

- Instruction accounts:
  - `[writable]` The program account to change the authority of.
  - `[signer]` The current authority of the program.
  - `[]` Optional, the reserved address for the next version of the program.
- Instruction data:
  - Enum variant `6u32`
- Behavior:
  - Check there are at least three instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account
  - Check that the status stored in the program account is deployed,
    otherwise throw `InvalidArgument`
  - for the program account of the next version
  (instruction account at index 2) check that:
    - the owner of the program account is loader-v4,
    otherwise throw `InvalidAccountOwner`
    - the program account is at least as long enough for the header,
    otherwise throw `AccountDataTooSmall`
    - the authority stored in the program account is the one provided,
    otherwise throw `IncorrectAuthority`
    - the status stored in the program account is not finalized,
    otherwise throw `Immutable`
  - Copy the address of the next version into the next version field stored in
  the previous versions program account
  - Change the status stored in the program account to finalized

## Impact

- This proposal covers all the use cases loader-v3 had but in a cleaner way and
comes with a specification.
- loader-v3 had a separate account type for buffers and extra commands for
these buffer accounts, in loader-v4 program accounts can act as buffers, there
is no more distinction.
- loader-v3 deployments always needed a buffer, in loader-v4 it is optional,
one can upload a redeployment into the program account directly.
- loader-v3 had two accounts per program, loader-v4 goes back to having only
one, thus needs less funds to reach rent exemption.
- loader-v3 closing of programs did finalize them, in loader-v4 all the funds
can be retrieved and the program address repurposed.
- loader-v3 programs could only grow, loader-v4 can shrink programs and also
retrieve the surplus of funds no longer required for rent exception.
- loader-v3 programs were always "live" after the first deployment, with
loader-v4 one can temporarily put a program into maintenance mode without a
redeployment.
- loader-v3 always required the entire program to be uploaded for a
redeployment, loader-v4 supports partial uploads for patching chunks of the
program.
- loader-v3 ELFs were misaligned, loader-v4 properly aligns the executable
file relative to the beginning of the account.
- loader-v4 allows finalized programs to mark which other program supersedes
them which can then be offered as an option in frontends. This provides a
more secure alternative to redeployment / upgrading of programs at the same
address. The keypair for the next version linked during finalization should be
generated beforehand.
- An option to migrate programs from loader-v3 to loader-v4 without changing
their program address will be available via a new loader-v3 instruction. (see
SIMD-0315)

## Security Considerations

None.

## Backwards Compatibility

None.
