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
per program. And has three account types: Program, program-data and buffer.
This was a workaround to circumvent the finality of the `is_executable` flag,
which will be ignored by the program runtime from SIMD-0162 onwards.
Consequentially, this setup of the program account being a proxy account,
containing the address of the actual program data account, is no longer
necessary and should be removed.

In loader-v3 every instruction which modified the program data had to re-verify
the ELF in the end. Instead we are now aiming for a more modular workflow which
explicates these steps in a Retract-Modify-Redeploy sequence of instructions
that allows multiple modifications to share one verification of the ELF in the
end. This sequence can be a single transaction or split across multiple
transactions.

Another issue with loader-v3 is that the executable file stored in the
programdata account is misaligned relative to the beginning of the account.
This currently requires a copy in the ELF loader to re-align the program.
To avoid any other alignment issues all states of the accounts owned by a
loader should have the same layout and only be differentiated by a status enum.

Additionally, there currently is no complete specification of the loaders
program management instructions. This proposal would thus fill that gap once
loader-v4 goes into production.

See impact for further motivation.

## Alternatives Considered

### Hot swap redeployment

A delay-visibility-free redeployment could be achieved by keeping the swap
program around until the end of the slot. This would however mean that two
accounts per program must be loaded until the dapp developer reclaims the
second one. That would defy the purpose of this proposal which is to get rid
of the proxy account.

### Adding a loader-v3 status

Instead of adding a loader-v3 status a new loader was added for the following
reasons:

- There is a lot of special casing throughout the validator for loader-v3,
which does not apply to loader-v4.
- The existing instruction interface of loader-v3 (e.g. the distinction
of program account and programdata account) are incompatible with loader-v4.
- Adding a new loader-v3 status would create new unforseen interactions with
the existing status enum and instructions.
- Modifying the existing loader-v3 implementation risks breaking it, while
a independent new loader does not.
- Loader-v3 is undocumented and unspecified. Starting fresh allows to have
a complete implementation, specification and documentation.

### Transfering closed accounts to the system program

While it would be more intuitive to give the program account back to the system
program immediately in the close instruction, this would also allow a program
to reopen itself under a different loader in the same transaction, while it is
still running. In order to prevent this the program account is only cleared and
the rent collection mechanism will then implicitly give the account back to the
system program at the very end of the transaction.

### Removing the redeployment cooldown

The cooldown enforces that each pair of an address and a slot can uniquely
identify a version of a program, which simplifies caching logic. Thus this
one slot cooldown will be kept in loader-v4.

### Inferring the program length from the ELF header

There are two issues with that: First, a ELF header does not know how long
its file is, it has to be parsed and calculated from offsets of various
sections, which increases the validator implementation complexity. Second,
this would require the ELF header to always be uploaded first and always in a
single chunk, wich increases the complexity of the uploading logic. Instead we
choose the user having to announce the length of the program to be uploaded
explicitly before the actual upload starts.

## New Terminology

The _current slot_ is as in the Clock sysvar.

Delay visibility: The changed version of a program only becomes available after
the current slot ends. Thus, the first transaction in the next slot can invoke
it.

Deployment cooldown: There can be at most one deployment per program in the
same slot. Subsequent deployments have to wait for the next slot.

## Detailed Design

The feature gate must enable loader-v4 program management and execution.

### Program Account Layout

Accounts of programs owned by loader-v4 must have the following layout:

- Header (which is 48 bytes long):
  - `u64` status enum:
    - Enum variant `0u64`: `Uninitalized`, account was zero-filled externally
    - Enum variant `1u64`: `NeverBeenDeployed`, used as write buffer
    - Enum variant `2u64`: `Retracted`, program is in maintenance
    - Enum variant `3u64`: `Deployed`, program is ready to be executed
    - Enum variant `4u64`: `Finalized`, same as `Deployed`, but can not be
    modified anymore
  - `u64` Slot in which the program was last deployed.
  - `[u8; 32]` Authority address which can send program management
  instructions.
- Body:
  - `[u8]` The programs executable file

### Program Account Header Verification

Verification the program account checks in the following order that:

- the owner of the program account is loader-v4,
otherwise throw `InvalidAccountOwner`
- the program account is at least as long enough for the header (48 bytes),
otherwise throw `AccountDataTooSmall`
- the program account is writable, otherwise throw `InvalidArgument`
- the provided authority (instruction account at index 1) signed,
otherwise throw `MissingRequiredSignature`
- the authority stored in the program account is the one provided,
otherwise throw `IncorrectAuthority`
- the status stored in the program account is not `Uninitalized`,
otherwise throw `InvalidArgument`
- the status stored in the program account is not `Finalized`,
otherwise throw `Immutable`

### Execution / Invocation

Invoking programs owned by loader-v4 checks in the following order that:

- the owner of the program account is loader-v4
- the program account is at least as long enough for the header
- the status stored in the program account is `Deployed` or `Finalized`
- the program account was not deployed within the current slot (delay
visibility)
- the executable file stored in the program account passes executable
verification

failing any of the above checks must throw `UnsupportedProgramId`.

### Program Management Instructions

The loader-v4 intructions Deploy and Retract are not authorized in CPI.

#### Initialize

- Instruction accounts:
  - `[writable]` The account to initialize as program account.
  - `[signer]` The new authority of the program.
- Instruction data:
  - Enum variant `0u32`
- Behavior:
  - Charge 32 CUs
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Check that the owner of the program account is loader-v4,
    otherwise throw `InvalidAccountOwner`
  - Check that the program account is at least as long enough for the header
    (48 bytes), otherwise throw `AccountDataTooSmall`
  - Check that the program account is writable,
    otherwise throw `InvalidArgument`
  - Check that the new authority (instruction account at index 1) has signed,
    otherwise throw `MissingRequiredSignature`
  - Check that the status stored in the program account is `Uninitalized`,
    otherwise throw `InvalidArgument`
  - Change the slot in the program account to the current slot
  - Change the status stored in the program account to `NeverBeenDeployed`
  - Copy the new authority address into the program account

#### SetAuthority

- Instruction accounts:
  - `[writable]` The program account to change the authority of.
  - `[signer]` The current authority of the program.
  - `[signer]` The new authority of the program.
- Instruction data:
  - Enum variant `1u32`
- Behavior:
  - Charge 32 CUs
  - Check there are at least three instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account header
  - Check that the new authority (instruction account at index 2) has signed,
    otherwise throw `MissingRequiredSignature`
  - Check that the current authority is different from the new authority,
    otherwise throw `InvalidArgument`
  - Copy the new authority address into the program account

#### Finalize

- Instruction accounts:
  - `[writable]` The program account to finalize.
  - `[signer]` The current authority of the program.
- Instruction data:
  - Enum variant `2u32`
- Behavior:
  - Charge 32 CUs
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account header
  - Check that the status stored in the program account is `Deployed` or
    that the status is `Retracted` and the program length is 0 (header only),
    otherwise throw `InvalidArgument`
  - Change the status stored in the program account to `Finalized`

#### SetProgramLength

- Instruction accounts:
  - `[writable]` The program account to change the size of.
  - `[signer]` The authority of the program.
- Instruction data:
  - Enum variant `3u32`
  - `u32` The new size after the operation.
- Behavior:
  - Charge 32 + new_size_in_bytes / cpi_bytes_per_unit CUs
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account header
  - Check that the status stored in the program account is
  `NeverBeenDeployed` or `Retracted`,
  otherwise throw `InvalidArgument`
  - Set the length of the program account to the requested new size plus
  the header size
  - Note: In CPI the maximum growth is limited to 10 KiB in ABI v1 and
  0 bytes in ABI v0.

#### Write

- Instruction accounts:
  - `[writable]` The program account to write to.
  - `[signer]` The authority of the program.
- Instruction data:
  - Enum variant `4u32`
  - `u32` Byte offset at which to write the given bytes
  - `[u8]` Chunk of the programs executable file
- Behavior:
  - Charge 32 + chunk_length_in_bytes / cpi_bytes_per_unit CUs
  - Check there are at least two instruction accounts,
  otherwise throw `NotEnoughAccountKeys`
  - Verify the program account header
  - Check the status stored in the program account is `NeverBeenDeployed` or
  `Retracted`,
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
  - `[]` The account to copy from.
- Instruction data:
  - Enum variant `5u32`
  - `u32` Byte offset at which to write
  - `u32` Byte offset at which to read
  - `u32` Length of the chunk to copy in bytes
- Behavior:
  - Charge 32 + chunk_length_in_bytes / cpi_bytes_per_unit CUs
  - Check there are at least three instruction accounts,
  otherwise throw `NotEnoughAccountKeys`
  - Check that program account and source account do not alias,
  otherwise throw `AccountBorrowFailed`
  - Verify the program account header
  - Check the status stored in the program account is `NeverBeenDeployed` or
  `Retracted`,
  otherwise throw `InvalidArgument`
  - Check that the source accounts owner and look-up the source header size:
    - loader-v2: 0 bytes
    - loader-v3 buffer: 37 bytes
    - loader-v3 programdata: 45 bytes
    - loader-v4: 48 bytes
    - if none of the above matches throw `InvalidArgument`
  - Check that the source end offset (sum of source offset and length) does
  not exceed the maximum (source account length minus the source header size),
  otherwise throw `AccountDataTooSmall`
  - Check that the destination end offset (sum of destination offset and
  length) does not exceed the maximum (program account length minus the loader-v4
  header size), otherwise throw `AccountDataTooSmall`
  - Copy the chunk between the program accounts at the offsets, each shifted by
  the header size of their loader (account owner) respectively

#### Deploy

- Instruction accounts:
  - `[writable]` The program account to deploy.
  - `[signer]` The authority of the program.
- Instruction data:
  - Enum variant `6u32`
- Behavior:
  - Charge 32 CUs
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account header
  - Check that the status stored in the program account is `NeverBeenDeployed`
  or `Retracted`
    otherwise throw `InvalidArgument`
  - Charge program_length_in_bytes / cpi_bytes_per_unit CUs
  - Check that the executable file stored in the program account passes
  executable verification
  - Change the slot in the program account to the current slot
  - Change the status stored in the program account to `Deployed`
  - Set the `is_executable` flag to `true`

#### Retract

- Instruction accounts:
  - `[writable]` The program account to retract.
  - `[signer]` The authority of the program.
- Instruction data:
  - Enum variant `7u32`
- Behavior:
  - Charge 32 CUs
  - Check there are at least two instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Verify the program account header
  - Check that the status stored in the program account is `Deployed`,
    otherwise throw `InvalidArgument`
  - Check that the slot stored in the program account is not the current
    (deployment cooldown), otherwise throw `InvalidArgument`
  - Note: The slot is **not** set to the current slot to allow a
  retract-modify-redeploy-sequence within the same slot or even within the
  same transaction.
  - Change the status stored in the program account to `Retracted`
  - Set the `is_executable` flag to `false`

#### WithdrawExcessLamports

- Instruction accounts:
  - `[writable]` The program account to withdraw from.
  - `[signer]` The authority of the program.
  - `[writable]` The recipient account.
- Instruction data:
  - Enum variant `8u32`
- Behavior:
  - Charge 32 CUs
  - Check there are at least three instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Check that program account and recipient account do not alias,
    otherwise throw `AccountBorrowFailed`
  - Check that the recipient account is writable,
  otherwise throw `InvalidArgument`
  - Verify the program account header, but skip the `Finalized` check
  - Transfer lamports which are not needed for rent exemption from the
  program account to the recipient account

#### EraseAndWithdrawAllLamports

- Instruction accounts:
  - `[writable]` The program account to withdraw from.
  - `[signer]` The authority of the program.
  - `[writable]` The recipient account.
- Instruction data:
  - Enum variant `9u32`
- Behavior:
  - Charge 32 CUs
  - Check there are at least three instruction accounts,
    otherwise throw `NotEnoughAccountKeys`
  - Check that program account and recipient account do not alias,
    otherwise throw `AccountBorrowFailed`
  - Check that the recipient account is writable,
  otherwise throw `InvalidArgument`
  - Verify the program account header
  - Check that the status stored in the program account is
  `NeverBeenDeployed` or `Retracted`,
  otherwise throw `InvalidArgument`
  - Set the length of the program account to 0 (removing the header too)
  - Transfer all lamports from the program account to the recipient account

### Workflows

#### Inital deployment

- Allocate an account to header plus ELF size
- Assign account to loader-v4
- Initialize to the new authority
- [Transaction boundary]
- Write chunks repeatedly
- [Transaction boundary]
- Deploy

#### Redeployment

- Allocate an account to header plus ELF size
- Assign buffer account to loader-v4
- Initialize of the buffer to the new authority
- [Transaction boundary]
- Write chunks repeatedly
- [Transaction boundary]
- Retract program
- SetProgramLength of program to ELF size
- Copy from buffer to program
- Deploy program
- EraseAndWithdrawAllLamports of buffer to program
- WithdrawExcessLamports of program

#### Close: Temporary

- Retract

#### Close: Recycle

- Retract
- EraseAndWithdrawAllLamports

#### Close: Permanent

- Retract
- SetProgramLength to 0
- WithdrawExcessLamports
- Finalize

#### Transfer authority

- SetAuthority to the new authority

## Impact

- This proposal covers all the use cases loader-v3 had but in a cleaner way and
comes with a specification.
- loader-v3 had a separate account layout for buffers and extra commands for
these buffer accounts, in loader-v4 they are only differentiated by status.
- loader-v3 had two accounts per program, loader-v4 goes back to having only
one, thus needs less funds to reach rent exemption.
- loader-v3 closing of programs did always finalize them, in loader-v4 there
is an option to retrieve all the funds the program address can be repurposed.
- loader-v3 programs could only grow, loader-v4 can shrink programs and also
retrieve the surplus of funds no longer required for rent exception.
- loader-v3 programs were always "live" after the first deployment, with
loader-v4 one can temporarily put a program into maintenance mode without a
redeployment.
- loader-v3 ELFs were misaligned, loader-v4 properly aligns the executable
file relative to the beginning of the account.
- An option to migrate programs from loader-v3 to loader-v4 without changing
their program address will be available via a new loader-v3 instruction. (see
SIMD-0315)

## Security Considerations

None.

## Backwards Compatibility

None.
