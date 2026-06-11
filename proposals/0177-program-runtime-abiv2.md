---
simd: '0177'
title: Program Runtime ABI v2
authors:
  - Alexander Meißner
  - Lucas Steuernagel
category: Standard
type: Core
status: Idea
created: 2025-02-23
feature: TBD
---

## Summary

Align the layout of the virtual address space to large pages in order to
simplify the address translation logic and allow for easy direct mapping.

## Motivation

- A better address space layout with lager alignment would allow for more
performant address translation, which is currently the bottleneck in execution.
- Programs should only have to pay for what they use. Work that can be done by
either the program runtime (eager) or the program (lazy) should be moved to the
program.
- Some restrictions and limits (such as the 10 KiB account growth limit in CPI)
could be lifted allowing precompiles and the remaining built-ins to be migrated
to core programs.
- The base CU cost of CPI could be significantly reduced as:
  - Most structures (instruction data, instruction account setup, return data)
  could be shared between programs and program runtime
  - Per instruction de/serialization on the runtime side could be eliminated
  - Deserialization inside the dApp could be reduced to a minimum
- Protocol complexity can be reduced, especially of the CPI logic
  - Work that only happens for CPI in ABIv0/v1:
    - Copy the instruction accounts (account meta)
    - Copy the transaction accounts (account info)
    - Copy the instruction data
    - Find the index of each provided pubkey by searching
    - Serialize/deserialize the transaction and instruction accounts
    - Verify the changes made by the callee
    - Apply the changes made by the callee
  - Work that happens for CPI in all ABIs:
    - Derive the signers from the seeds
    - Deduplicate the instruction accounts
    - Consolidate the permissions of the instruction accounts
    - Check for priviledge escalations in the instruction accounts
    - Adjust access rights to memory mappings

## Alternatives Considered

### Updating transaction account metadata via direct writing vs syscalls

The ABIv0/v1 approach of letting programs modify the account metadata by
writing to them directly can be implemented in three ways:

- Serialization and deserialization, which incurs work for all accounts at
every instruction boundary.
- Many small mappings for each field of every account, which also have to be
updated at every instruction boundary.
- A memory access violation handler, which is very similar to the syscall
approach except that it lacks proper error codes. Also, for wide fields such as
the 32 bytes owner field it would result in multiple calls, thus being less
efficient than a single syscall.

Additionally, some fields must be verified after being written to, such as the
lamports field which for readonly accounts can only increase, and others like
the owner field can trigger additional changes such as a writable account
turning into a readonly account. Because the program runtime does not know
which field of which account was touched it has to check all of them, which is
wasteful as most transactions do not touch all accounts.

With using syscalls to update account metadata fields instead there is some
overhead in the actual modification. It is however a lot less work in total as
all the unaffected accounts don't incur any work.

### Account visibility, when not passed as instruction account

Since there is address space for all transaction accounts, the question arises
what should happen to those which are not covered by at least one instruction
account of the currently executing instruction.

- Completely hide them, both the metadata and the payload. This would require
many small mappings for every account, which have to be updated at every
instruction boundary.
- Hide only the payload, but show the metadata as readonly. This would require
only a single memory mapping update per account at every instruction boundary.
- Show both the metadata and the payload as readonly. This does not require any
additional memory mapping updates at instruction boundaries.

Since accounts are public and known to everybody anyway, there is no point in
performing extra work to hide them. However, this also means that accounts can
be read without being passed through the CPI stack as instruction accounts,
which is particularly useful for sysvar accounts. Another consequence is that
it becomes possible for a caller to reference readonly non-signer accounts by
their index in the instruction data instead of going through the instruction
accounts interface.

### Representation of accounts when they are syscall parameters

There are multiple options when it comes to passing accounts as syscall
parameters:

- as pubkey slice, which is what ABIv0/v1 does and is inefficient. It requires
the runtime to resolve it to an account index, as that is what is used
internally.
- as reference / pointer, which also requires the runtime to convert them back
to indices, but is a lot cheaper than the pubkey lookup. It has the advantage
that it does not only work for accounts but other buffers such as the CPI
scratchpads or the return data scratchpad too.
- as index in transaction or as index in instruction. The runtime needs both,
the index in instruction to verify the permissions and index in transaction to
apply the change to the transaction account, but can convert them back and
forth easily. Thus, the only question is whether the as index in transaction or
as index in instruction is easier for programs to handle.

### General purpose pubkey lookup mechanism

Instead of just fixing sysvars at specific addresses it might be of more
general interest to resolve a pubkey to transaction account index without
having to scan the transaction account list. This might also be useful to
programs which hold pubkeys in the payload of accounts and need to match them
against the current transaction. Additionally, fixing account indices poses
an obstacle to other protocol users of the SVM when expanding the set of
sysvars as the same index can then collide in different protocols.

## New Terminology

None.

## Detailed Design

Programs signal that they expect ABIv2 through their SBPF version field being
v4 or above.

### Memory Regions

The virtual address space layout is split into memory regions, each 4 GiB in
size, starting at the following addresses:

- `0x000000000`: SBPFv3 program read only data
- `0x100000000`: SBPFv3 program byte code
- `0x200000000`: SBPFv3 stack
- `0x300000000`: SBPFv3 heap
- `0x400000000`: [Transaction metadata](#transaction-metadata)
- `0x500000000`: [Transaction account](#transaction-account)
- `0x600000000`: [Instruction trace](#instruction-trace)
- `0x700000000`: [Return data scratchpad](#return-data-scratchpad)
- `0x800000000 + transaction_account_index * 0x100000000`: [Transaction account payload](#transaction-account-payload)
- `0x100000000000 + instruction_index * 0x100000000`: [Instruction payload](#instruction-payload)
- `0x104000000000 + instruction_index * 0x100000000`: [Instruction accounts](#instruction-accounts)
- `0x108000000000`: END

This limits the maximum number of the following we could ever support:

- Accounts (including sysvars) in a transaction to 4088
- Accounts in an instruction to 65536
- Instructions in a transaction to 64
- Bytes in the instruction payload to 4 GiB
- Bytes in the account payload to 4 GiB

#### Transaction metadata

At the beginning of a transaction the program runtime must prepare a readonly
memory region. This region is shared by all instructions running programs with
support to new ABI. It must be updated as as instructions through out the
transaction modify the CPI scratchpad or the return data. The contents of this
memory region are the following:

- Public key / address of the program which wrote to the return-data scratchpad
  most recently: `[u8; 32]`
- The return-data scratchpad: `&[u8]`, which is composed of:
  - Pointer to return-data scratchpad: `u64`
  - Length of return-data scratchpad: `u64`
- The CPI scratchpad: `&[u8]`, which consists of:
  - Pointer to CPI scratchpad: `u64`
  - Length of CPI scratchpad: `u64`
- The CPI accounts scratchpad: `&[InstructionAccount]`, which consists of:
  - Pointer to slice: `u64`
  - Number of elements in slice: `u64`
- Index of current executing instruction: `u32`
- Total number of instructions in transaction (including CPIs and top level
  instructions): `u32`
- Number of CPIs in trace (under execution and finished): `u32`
- The number of transaction accounts: `u32`

As accounts in this area sorted by their index in transaction, the payer
account must always be the account at index zero, as we surface the internal
account ordering to programs.

#### Transaction account

This memory region is readonly and holds the metadata for all accounts in the
transaction. It is shared by all instructions running programs with support for
ABIv2, and must be updated as instruction modify the metadata with the provided
syscalls (see the `Changes to syscalls` section). The contents for this region
are as follow:

- For each `TransactionAccount`:
  - Public key / address of itself: `[u8; 32]`
  - Public key / address of the owner: `[u8; 32]`
  - Lamports: `u64`
  - [Transaction account payload](#transaction-account-payload): `&[u8]`
  which consists of:
    - Pointer to account payload: `u64`
    - Account payload length: `u64`

#### Instruction trace

This memory region is readonly and must be updated at each intruction
invocation. The contents of this region are the following:

- For each instruction in transaction:
  - Reserved filed for alignment and potential future usage: `u16`
  - Index in transaction of program account to be executed: `u16`
  - CPI nesting level: `u16`
  - Index of parent instruction (`u16::MAX` for top-level instructions): `u16`
  - Reference to a slice of instruction accounts `&[InstructionAccount]`,
    consisting of:
    - Pointer to slice: `u64`
    - Number of elements in slice: `u64`
  - Instruction payload `&[u8]`, which is composed of:
    - Pointer to data: `u64`
    - Length of data: `u64`

#### Return data scratchpad

This memory region holds the return-data of the transaction and starts out as
readonly but can temporarily become writable until the next instruction edge.
When it becomes writable the corresponding fields in
[Transaction metadata](#transaction-metadata) must be updated.
See [Scratchpad management](#scratchpad-management).

#### Transaction account payload

For each transaction account one separate memory region must be mapped to its
data (as opposed to its metadata).

Only if the instruction account has the writable flag set and is owned by the
current program it is mapped in as writable. The writability of a region must
be updated as programs through out the transaction modify the account metadata
or as the program (and thus owner relation) changes between instructions.

#### Instruction payload

For each transaction account one separate readonly memory region must be mapped
to its data (as opposed to its metadata).

One additional writable memory region can be created after the last instruction
as zero-copy CPI scratch pad. See [Scratchpad management](#scratchpad-management).

#### Instruction accounts

For each transaction account one separate readonly memory region must be mapped
to the list of its `InstructionAccount`s, each consisting of:

- Index to transaction account: `u16`
- Signer flag: `u8` (1 for signer, 0 for non-signer)
- Writable flag: `u8` (1 for writable, 0 for readonly)

One additional writable memory region can be created after the last instruction
as zero-copy CPI scratch pad. See [Scratchpad management](#scratchpad-management).

### VM initialization

During the initilization of the virtual machine, the runtime must load the
following values to registers:

1. Register R1: A pointer to the metadata of the instruction under execution.
   (see section [Instruction trace](#instruction-trace)).
2. Register R2: A pointer to the instruction accounts slice for the
   instruction under execution (see section
   [Instruction accounts](#instruction-accounts)).
3. Register R3: The number of instruction accounts for the instruction under
   execution.
4. Register R4: A pointer to the instruction payload of the instruction under
   execution (see section [Instruction payload](#instruction-payload)).
5. Register R5: The payload length for the instruction under execution.

### Changes to syscalls

#### Added syscalls

Changes to the account metadata must now be communicated with specific
syscalls, as detailed below:

- `sol_assign_owner(u64, *const [u8; 32])`.
  - `u64`: Index in transaction of the account whose owner is changing,
  - `*const [u8; 32]`: Pointer to the public key of the new owner.
- `sol_transfer_lamports(u64, u64, u64)`:
  - `u64`: Index in transaction of the destination account.
  - `u64`: Index in transaction of the source account.
  - `u64`: Lamports amount.

Changes to the account payload length and all the scratchpads sections
introduced in this SIMD (the return-data scratchpad and the CPI scratchpad)
must be communicated via a new sycall `set_buffer_length(u64, u64)`, with the
following parameters:

- `u64`: Base address of the memory region to be resized.
- `u64`: New length of the memory region.

The syscall must start by charging a base cost (to be determined) plus the
same CU per byte ratio as the `memset` syscall for the new length of the memory
region. Then it must check if the address matches the base address of either a
writable account payload mapping or one of the scratchpad mappings and return
an error otherwise. Constrains for the maximum resizable limits must also be
verified for each region separetely.

#### CPI

The parameters for `sol_invoke_signed_v2` are the following:

1. Index of the [Transaction account](#transaction-account) of the callee: `u64`
2. A pointer to the singer seeds of type `VmSlice<VmSlice<VmSlice<u8>>>`
3. The length of the outer signer seeds slice in
  `VmSlice<VmSlice<VmSlice<u8>>>`

`VmSlice<T>` is a stable layout type defined to share slices between the guest
and the host. It consists of:

- `u64`: Pointer to the data.
- `u64`: Length of data (number of elements `T`)

With the new `sol_invoke_signed_v2` syscall, CPIs must be managed
differently. At each CPI call, the runtime must perform the following actions:

1. Verify that all account indexes received in the `InstructionAccount` area
  belong in the current executing instruction. Likewise, the prgram ID index
  that should be called must also undergo the same verification.
2. Verify that accounts have the correct signer and writer flags set, avoiding
  privelege promotion.
3. Append a new instruction at the end of the
  [Instruction-trace](#instruction-trace).
4. Transform the caller CPI scratchpad into a readonly instruction payload
  region visible for the callee.
5. Change the read and write permission for the
  [Transaction-account](#transaction-account) regions.
6. Update the address for the callee CPI scratchpad, the index of current
  executing instruction, and the number of instructions in transaction
  in the [Transaction-metadata](#transaction-metadata).

When the CPI returns, the runtime must do the following:

1. Bump the address for the CPI scratchpad, and keep the one which was used for
  the callee in its exisiting address assigned during CPI call.
2. Change the read and write permission for the
  [Transaction-account](#transaction-account) regions.
3. Update the index of current executing instruction.

CPIs between ABIv0/v1 and ABIv2 program must be allowed, but costs will difer.
A CPI from an ABIv2 to another ABIv2 program must cost less than a CPI from an
ABIv2 to an ABIv0/v1 program, due to the decreased work overhead from program
runtime.

### Scratchpad management

This SIMD introduces memory regions referred to as scratchpads:

1. The [return data scratchpad](#return-data-scratchpad)
2. The last memory region of [Instruction payload](#instruction-payload)
3. The last memory region of [Instruction accounts](#instruction-accounts)

The first one becomes readonly at every instruction edge but retains its data
and the other two are re-mapped to become empty at every instruction edge.
Thus a program has to initialize them by calling the `set_buffer_length`
syscall if it wishes to write to them.

### Sysvar accounts

For each of the following sysvars, the runtime must map load them as
transaction accounts regardless of their pubkey being mentioned in the message:

-1. Clock
-2. Epoch rewards
-3. Epoch schedule
-4. Last restart slot
-5. Rent
-6. Slot hashes
-7. Stake history

These must be mapped at the end of the address space reserved for
[Transaction account payload](#transaction-account-payload), leaving a gap to
the other transaction accounts. Corresponding
[Transaction account](#transaction-account) must be serialized too. If some of
these sysvars are mentioned in the message, they are to be filtered out and also
be mapped at the end, such that the order and indices are remain stable.

### Changes to CU metering

CPI will no longer charge CUs for the length of account payloads. Instead TBD
CUs will be charged for every instruction account. Also TBD CUs will be charged
for the three new account metadata updating syscalls. TBD will be charged for
resizing a scratchpad.

## Impact

- Full interoperability between all ABI versions in all directions (CPI between
ABI v0, v1 and v2).
- CU cost of instruction accounts and CPI will be reduced significantly.
- Unrestricted account resizing will become possible, as long as all parent
instructions are ABIv2 as well.
- In turn this means the account creation pattern of resize-then-reassign can
finally be corrected to become reassign-then-resize.
- All transaction accounts (including sysvars) will become accessible to all
instructions (read-only) all the time.
- All top-level instructions (including not yet executed ones) will become
visible all the time.
- CPI caller restrictions will remain in place: Both for the callee program as
well as instruction accounts.
- ABIv0/v1 offered virtual address space stability across transactions if the
instruction accounts were exactly the same (order and duplicates / aliasing).
This is not the case in ABIv2 and relative instead of absolute addressing must
be used for structures inside of account payloads.

### Lazy deserialization on the dApp side (inside the SDK)

With this design a program SDK can (but no longer needs to) eagerly deserialize
all account metadata at the entrypoint. Because this layout is strictly aligned
and uses proper arrays, it is possible to directly calculate the offset of a
single accounts metadata with only one indirect lookup and no need to scan all
preceeding metadata. This allows a program SDK to offer a lazy interface which
only interacts with the account metadata fields which are needed, only of the
accounts which are of interest and only when necessary.

### Migration guide

dApps / SDKs will have to adapt:

- to calling `sol_transfer_lamports` instead of reading and adjusting the
  lamports fields of both accounts separately.
- to calling `sol_assign_owner` instead of writing to the account owner field
  directly.
- to calling `sol_set_return_data` instead of writing to the account length
  field directly.
- the fact that account metadata updates happen immediately (just like the
  account payload) and are no longer delayed until the next instruction edge.
- reading the [Instruction trace](#instruction-trace) directly instead of
  calling `sol_get_processed_sibling_instruction`.
- reading and writing the [return data scratchpad](#return-data-scratchpad)
  directly instead of calling `sol_get_return_data`.
- calling `set_buffer_length` instead of `sol_set_return_data`.
- read sysvars from a
  [Transaction account payload](#transaction-account-payload) memory region
  instead of using the sysvar syscalls.
- the CPI workflow to:
  1. `set_buffer_length` the last
    [Instruction accounts](#instruction-accounts) memory region.
  2. write to the last [Instruction accounts](#instruction-accounts) memory
  region.
  3. `set_buffer_length` the last
    [Instruction payload](#instruction-payload) memory region.
  4. write to the last [Instruction payload](#instruction-payload) memory
  region.
  5. pass the callee program and signer seeds in `sol_invoke_signed_v2`.

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks

This will require parallel code paths for serialization, deserialization, CPI
call edges and CPI return edges. All of these will coexist with the exisiting
ABI v0 and v1 for the forseeable future, until we decide to deprecate them.
It might be possible to turn ABIv0/v1 de/serialization into a core program, but
their CPI syscalls will be hard to port.
