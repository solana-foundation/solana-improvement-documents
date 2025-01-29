---
simd: '0167'
title: Loader-v4
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Review
created: 2024-08-15
feature: TBD
---

## Summary

A new upgradeable loader which only requires a single account per program.

## Motivation

Loader-v3, which is currently the only deployable loader, requires two accounts
per program. This was a workaround to circumvent the finality of the
`is_executable` flag, which is removed in SIMD-0162. Consequentially, this
setup of the program account being a proxy account, containing the address of
the actual programdata account, is no longer necessary and should be removed.

Another issue with loader-v3 is that the executable file stored in the
programdata account is misaligned relative to the beginning of the account.

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

The associated feature gate must:

- add loader-v4 to the write lock demotion exceptions
- enable loader-v4 `LoaderV411111111111111111111111111111111111` program
management and execution
- simultaneously disable new deployments on loader-v3
(`BPFLoaderUpgradeab1e11111111111111111111111`),
throwing `InvalidIstructionData` if `DeployWithMaxDataLen` is called.

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
  - `u32` Offset at which to write the given bytes
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

#### SetProgramLength

- Instruction accounts:
  - `[(signer), writable]` The program account to change the size of.
  - `[signer]` The authority of the program.
  - `[writable]` Optional, the recipient account.
- Instruction data:
  - Enum variant `1u32`
  - `u32` The new size after the operation.
- Behavior:
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - If this is an initialization (program account length is too short to
  contain the header and the requested new size is greater 0):
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
  - `[writable]` Optional, an undeployed source program account to take data
  and lamports from.
- Instruction data:
  - Enum variant `2u32`
- Behavior:
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account
  - Check that the slot stored in the program account is not the current
  (deployment cooldown), otherwise throw `InvalidArgument`
  - Check that the status stored in the program account is retracted
    otherwise throw `InvalidArgument`
  - In case a source program was provided (instruction account at index 2):
    - Verify the source program account
    - Check that the status stored in the source program account is retracted,
    otherwise throw `InvalidArgument`
    - Check that the executable file stored in the source program account
    passes executable verification
      - The feature set that the executable file is verified against is not
      necessarily the current one, but the one of the epoch of the next slot
      - Also, during deployment certain deprecated syscalls are disabled,
      this stays the same as in the older loaders
    - Copy the entire source program account into the program account
    - Set the length of the source program account to zero
    - Transfer all funds of the  source program account to the program
    account
  - In case no source program was provided:
    - Check that the executable file stored in the program account passes
    executable verification
  - Change the slot in the program account to the current slot
  - Change the status stored in the program account to deployed

#### Retract

- Instruction accounts:
  - `[writable]` The program account to retract.
  - `[signer]` The authority of the program.
- Instruction data:
  - Enum variant `3u32`
- Behavior:
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account
  - Check that the slot stored in the program account is not the current
  (deployment cooldown), otherwise throw `InvalidArgument`
  - Check that the status stored in the program account is deployed,
    otherwise throw `InvalidArgument`
  - Note: The slot is **not** set to the current slot to allow a
  retract-modify-redeploy-sequence within the same slot
  - Change the status stored in the program account to retracted

#### TransferAuthority

- Instruction accounts:
  - `[writable]` The program account to change the authority of.
  - `[signer]` The current authority of the program.
  - `[signer]` The new authority of the program.
- Instruction data:
  - Enum variant `4u32`
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
  - Enum variant `5u32`
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

This proposal:

- covers all the use cases loader-v3 had but in a cleaner way and comes with
a specification.
- allows finalized programs to mark which other program supersedes them which
can then be offered as an option in forntends. This provides a more secure
alternative to redeployment / upgrading of programs at the same address.
- makes deployment slightly cheaper for dapp developers as they would no longer
have to burn funds for the rent exception of the proxy account.
- provides an alternative redeployment path which does not require a big
deposit of funds for rent exception during the upload.
- enables dapp developers to withdrawl the surplus of funds required for rent
exception when shortening the length of program accounts or closing them.
- shortens the workflow of temporarily closing a program to a single
instruction, instead of having to build and redeploy an empty program.
- properly alignes the executable file relative to the beginning of the
account. In loader-v3 it is misaligned.
- once all loader-v3 programs are migrated:
  - allows transaction account loading to be simplifed, because every program
  would load exactly one account, no need to load the proxy account to get to
  the actual program data (which is not listed in the transaction accounts).
  - allows the removal of the write lock demotion exception if loader-v3 is
  present in a transaction.
  - corrects the miscounting of the proxy account size towards the total
  transaction account loading limit.

## Security Considerations

None.

## Backwards Compatibility

This proposal does not break any existing programs. However, dapp developers
might want to profit from the new program mangement instructions without
influencing their users work flows. To do so they would need a way to turn the
program accounts of loader-v3 to program accounts of loader-v4, changing the
account owner but keeping the program address. A potential issue is that the
programdata header of loader-v3 is only 45 bytes long while loader-v4 takes 48
bytes. An automatic mechanism in the program runtime (triggered by feature
activation) could then perform the following steps per program:

- loader-v3 clears the program proxy account (setting its size to zero)
- loader-v3 transfers all funds from the programdata to the proxy account
- loader-v3 gifts the program proxy account to loader-v4
- loader-v4 initializes it via `Truncate`
- loader-v4 copies the data from the programdata account via `Write`
- loader-v4 deploys it via `Deploy`
- Optinally, loader-v4 finalizes it without a next version forwarding
- loader-v3 closes the programdata account (setting its size to zero)
