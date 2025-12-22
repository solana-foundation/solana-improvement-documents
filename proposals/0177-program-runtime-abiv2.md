---
simd: '0177'
title: Program Runtime ABI v2
authors:
  - Alexander Meißner
  - Lucas Stuernagel
category: Standard
type: Core
status: Idea
created: 2025-02-23
feature: TBD
extends: SIMD-0219
---

## Summary

Align the layout of the virtual address space to large pages in order to
simplify the address translation logic and allow for easy direct mapping.

## Motivation

Direct mapping of the account payload data is enabled by SIMD-0219.
However, there remains a big optimization potential for both programs and the
program runtime:

- Instruction data could be mapped directly as well
- Return data could be mapped directly too
- Account payload could be resized freely (no more 10 KiB growth limit)
- CPI could become cheaper in terms of CU consumption
- Most structures could be shared between programs and program runtime,
requiring only a single serialization at the beginning of a transaction and
only small adjustments after
- Per instruction serialization before a program runs could be removed entriely
- Per instruction deserialization after a program runs could be removed too
- Deserialization inside the dApp could be reduced to a minimum
- programs would only have to pay for what they use, not having to deserialize
all instruction accounts which were passed in
- Scanning sibling instructions would not require a syscall
- Memory regions (and thus address translation) which SIMD-0219 made unaligned
could be aligned (to 4 GiB) again

All of these however do necessitate a major change in the layout how the
program runtime and programs interface (ABI).

## Alternatives Considered

None.

## New Terminology

None.

## Detailed Design

Programs signal that they expect ABIv2 through their SBPF version field being
v4 or above.

### Memory Regions

#### Transaction metadata area

At the beginning of a transaction the program runtime must prepare a 
readonly memory region starting at `0x400000000`. This region is shared by all 
instructions running programs with support to new ABI. It must be updated as
as instructions through out the transaction modify the CPI scratchpad or the 
return data. The contents of this memory region are the following:

- Key of the program which wrote to the return-data scratchpad most 
  recently: `[u8; 32]`
- The return-data scratchpad: `&[u8]`, which is composed of:
  - Pointer to return-data scratchpad: `u64`
  - Length of return-data scratchpad: `u64`
- The CPI scratchpad: `&[u8]`, which consists of:
  - Pointer to CPI scratchpad: `u64`
  - Length of CPI scratchpad: `u64`
- Index of current executing instruction: `u32`
- Number of instructions in transaction (including CPIs): `u32`
- Number of executed CPIs: `u32`
- The number of transaction accounts: `u32`


#### Account metadata area

This region starts at `0x500000000`, is readonly and holds the metadata for 
all accounts in the transaction. It is shared by all instructions running 
programs with support for ABIv2, and must be updated as instruction modify the 
metadata with the provided syscalls (see the `Changes to syscalls` section).
The contents for this region are as follow:

- For each transaction account:
  - Key: `[u8; 32]`
  - Owner: `[u8; 32]`
  - Lamports: `u64`
  - Account payload: `&[u8]` which is composed of:
    - Pointer to account payload: `u64`
    - Account payload length: `u64`

#### Instruction area

For each transction, the program runtime must also preapre two memory regions. 
The first one is a readonly region starting at `0x600000000`. It must be 
updated at each CPI call edge. The contents of this region are the following:

- For each instruction in transaction:
  - Index in transaction of program account to be executed: `u64`
  - CPI nesting level: `u32`
  - Index of parent instruction (`u32::MAX` for top-level instructions): `u32`
  - Reference to a slice of instruction accounts `&[InstructionAccount]`, 
    consisting of:
    - Pointer to slice: `u64`
    - Number of elements in slice: `u64`
  - Instruction data `&[u8]`, which is composed of:
    - Pointer to data: `u64`
    - Length of data: `u64`

Let `InstrucionAccount` contain the following fields:

  - Index to transaction account: `u16`
  - Signer flag: `u8` (1 for signer, 0 for non-singer)
  - Writable flag: `u8` (1 for writable, 0 for readonly)

#### Return data scratchpad

A writable memory region starting at `0x700000000` must be mapped in for the
return-data scratchpad.

### Accounts area

For each unique (meaning deduplicated) instruction account the payload must
be mapped in at `0x800000000` plus `0x100000000` times the index of the
**transaction** account (not the index of the instruction account). Only if the
instruction account has the writable flag set and is owned by the current
program it is mapped in as a writable region. The writability of a region must
be updated as programs through out the transaction modify the account metadata.

The runtime must only map the payload for accounts that belong in the current
executing instruction. The payload for accounts belonging to sibling instructions
must NOT be mapped.

### Instruction payload area

For each instruction, the runtime must map its payload at address 
`0x10800000000` plus `0x100000000` times the index of the instruction in the 
trasaction. All instruction payload mappings are readonly.

One extra writable mapping must be created after the last instruction payload 
area to be the CPI scratch pad, i.e. at address `0x10800000000` plus 
`0x100000000` times the number of instructions in the transaction. Its purpose  
is for programs to write CPI instruction data directly to it and avoid copies.

### Instruction accounts area

For each instruction, the runtime must map an array of `InstructionAccount`
(as previously defined) at address `0x14800000000` plus `0x100000000` times 
the index of the instruction in the transaction. This mapped are is readonly.

Each of these memory regions contain the following for each instruction:

- For each account in instruction:
  - `InstructionAccount`, consisting of:
    - Index to transaction account: `u16`
      - Signer flag: `u8` (1 for signer, 0 for non-singer)
      - Writable flag: `u8` (1 for writable, 0 for readonly)

### VM initialization

During the initilization of the virtual machine, the runtime must load the 
value `0x400000000` in register `R1`, value `0x500000000` in register `R2`, 
and value `0x600000000` in register `R3`. These values represent addresses for 
programs to easily find the areas to read information about the transaction 
and the instructions.

### Changes to syscalls

Changes to the account metadata must now be communicated with specific 
syscalls, as detailed below:

- `sol_assign_owner`: Dst account, new owner as `&[u8; 32]`
- `sol_transfer_lamports`: Dst account, src account, amount as `u64`

The account parameters are the index of the account in the transaction.

Changes to the account payload length and all the scratchpads sections 
introduced in this SIMD (the return-data scratchpad and the CPI scratchpad) 
must be communicated via a new sycall `set_buffer_length`, with the following 
parameters:

- Address of region to be resized: `u64`
- New length of region: `u64`

The syscall must check if the address belongs to either a writable account 
payload or one of the scratchpads and return and error otherwise. Constrains 
for the maximum resizable limits must also be verified (10 kb).

The verifier must reject SBPFv4 programs containing the `sol_invoke_signed_c` 
and `sol_invoke_signed_rust`, since they are not compatible with ABIv2. A new 
syscall `sol_invoke_signed_v2` must replace them. The parameters for 
`sol_invoke_signed_v2` are the following:

- Index in transaction of program ID to be called: `u64`.
- A pointer to a slice `&[InstructionAccount]`, with each element 
  `InstructionAccount` 
  containing, as previously mentioned:
  - Index to transaction account: `u16`
    - Signer flag: `u8` (1 for signer, 0 for non-singer)
    - Writable flag: `u8` (1 for writable, 0 for readonly)
- The length of the `&[InstructionAccount]` slice.
- A pointer to the singer seeds of type `&[&[&[u8]]]`.
- The length of the outer signer seeds slice in `&[&[&[u8]]]`.

Programs using `sol_get_return_data` and `sol_set_return_data` must be 
rejected by the verfier if ABI v2 is in use.

### Scratchpads managemnts

This SIMD introduces two scratch pad regions: the return-data scratchpad and 
the CPI scratchpad. At the beginning of every instruction, these scratchpads 
must be empty and their size must be zero.

Programs must set the desired length for them using the `set_buffer_length` 
syscall. Reads and writes to a region beyond the scratchpad length must 
trigger an access violation error.

The management for the writable accounts payload must work similarly, except 
that they must not be initialized empty, but instead with the pre-existing 
data it holds.

### CPIs

With ABIv2 and the new `sol_invoke_signed_v2` syscall, CPIs must be managed 
differently. At each CPI call, the runtime must perform the following actions:

1. Verify that all account indexes received in the `InstructionAccount` array 
   belong in the current executing instruction. Likewise, the prgram ID index 
   that should be called must also undergo the same verification.
2. Append the slice `&[InstructionAccount]` passed as a parameter to the 
   array kept at address `0x700000000`.
3. Append a new instruction at the end of the serialization array kept at 
   `0x600000000`.
4. Transform the caller CPI scratchpad into a readonly instruction payload 
   region visible for the callee.
5. Change the visibility and write permissions for the account payload 
   regions, according to the CPI accounts and their flags.
6. Update the address for the callee CPI scratchpad, the index of current 
   executing transaction, and the number of instructions in transaction at 
   address `0x400000000`.

When the CPI returns, the runtime must do the following:

1. Update the address for the CPI scratchpad, and keep the previouly used one 
   in its exsiting address assigned during CPI call. The new CPI scratchpad 
   address is the same as the previous one plus `0x100000000`.
2. Change the read and write permission for the account payload regions, 
   according to potential changes in account ownership.
3. Update the index of current executing instruction.
4. No changes must be done in addresses `0x600000000` and `0x700000000`.

### Changes to CU metering

CPI will no longer charge CUs for the length of account payloads. Instead TBD
CUs will be charged for every instruction account. Also TBD CUs will be charged
for the three new account metadata updating syscalls. TBD will be charged for
resizing a scratchpad.

### Lazy deserialization on the dApp side (inside the SDK)

With this design a program SDK can (but no longer needs to) eagerly deserialize
all account metadata at the entrypoint. Because this layout is strictly aligned
and uses proper arrays, it is possible to directly calculate the offset of a
single accounts metadata with only one indirect lookup and no need to scan all
preceeding metadata. This allows a program SDK to offer a lazy interface which
only interacts with the account metadata fields which are needed, only of the
accounts which are of interest and only when necessary.

## Impact

This change is expected to drastically reduce the CU costs as the cost will no
longer depend on the length of the instruction account payloads or instruction
data.

From the dApp devs perspective almost all changes are hidden inside the SDK.

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks

This will require parallel code paths for serialization, deserialization, CPI
call edges and CPI return edges. All of these will coexist with the exisiting
ABI v0 and v1 for the forseeable future, until we decide to deprecate them.
