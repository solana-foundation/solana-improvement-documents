---
simd: '0177'
title: Program Runtime ABI v2
authors:
  - Alexander Meißner
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

At the moment all validator implementations have to copy (and compare) data in
and out of the virtual memory of the virtual machine. There are four possible
account data copy paths:

- Serialization: Copy from program runtime (host) to virtual machine (guest)
- CPI call edge: Copy from virtual machine (guest) to program runtime (host)
- CPI return edge: Copy from program runtime (host) to virtual machine (guest)
- Deserialization: Copy from virtual machine (guest) to program runtime (host)

To avoid this a feature named "direct mapping" was designed which uses the
address translation logic of the virtual machine to emulate the serialization
and deserialization without actually performing copies.

Implementing direct mapping in the current ABI v0 and v1 is very complex
because of unaligned virtual memory regions and memory accesses overlapping
multiple virtual memory regions. Instead the layout of the virtual address
space should be adjusted so that all virtual memory regions are aligned to
4 GiB.

## Alternatives Considered

None.

## New Terminology

None.

## Detailed Design

Programs signal their support through their SBPF version field being v4 or
above while the program runtime signals which ABI is chosen through the
serialized magic field.

### Per Transaction Serialization

At the beginning of a transaction the program runtime must prepare the
following which is shared by all instructions running programs suporting the
new ABI. This memory region starts at `0x400000000` and is readonly. It must be
updated as instructions through out the transaction modify the account metadata
or the scratchpad via `sol_set_return_data`.

- Key of the program which wrote to the scratchpad most recently: `[u8; 32]`
- The scratchpad data: `&[u8]` which is composed of:
  - Pointer to scratchpad data: `u64`
  - Length of scratchpad data: `u64`
- The number of transaction accounts: `u64`
- For each transaction account:
  - Key: `[u8; 32]`
  - Owner: `[u8; 32]`
  - Lamports: `u64`
  - Account payload: `&[u8]` which is composed of:
    - Pointer to account payload: `u64`
    - Account payload length: `u64`

A readonly memory region starting at `0x500000000` must be mapped in for the
scratchpad data. It must be updated when `sol_set_return_data` is called.

### Per Instruction Serialization

For each instruction the program runtime must prepare the following.
This memory region starts at `0x600000000` and is readonly. It does not require
any updates once serialized.

- The instruction data: `&[u8]` which is composed of:
  - Pointer to instruction data: `u64`
  - Length of instruction data: `u64`
- Programm account index in transaction: `u16`
- Number of instruction accounts: `u16`
- For each instruction account:
  - Index to transaction account: `u16`
  - Flags bitfield: `u16` (bit 0 is signer, bit 1 is writable)

### Per Instruction Mappings

A readonly memory region starting at `0x700000000` must be mapped
in for the instruction data. It too does not require any updates.

For each unique (meaning deduplicated) instruction account the payload must
be mapped in at `0x800000000` plus `0x100000000` times the index of the
**transaction** account (not the index of the instruction account). Only if the
instruction account has the writable flag set and is owned by the current
program it is mapped in as a writable region. The writability of a region must
be updated as programs through out the transaction modify the account metadata.

### Lazy deserialization on the dApp side (inside the SDK)

With this design a program SDK can (but no longer needs to) eagerly deserialize
all account metadata at the entrypoint. Because this layout is strictly aligned
and uses proper arrays, it is possible to directly calculate the offset of a
single accounts metadata with only one indirect lookup and no need to scan all
preceeding metadata. This allows a program SDK to offer a lazy interface which
only interacts with the account metadata fields which are needed, only of the
accounts which are of interest and only when necessary.

### Changes to syscalls

The `AccountInfo` parameter of the CPI syscalls (`sol_invoke_signed_c` and
`sol_invoke_signed_rust`) will be ignored if ABI v2 is in use. Instead the
changes to account metadata will be communicated explicitly through separate
syscalls `sol_set_account_owner`, `sol_set_account_lamports` and
`sol_set_account_length`. Each of these must take a guest pointer to the
structure of the transaction account (see per transaction serialization) to be
updated and the new value as second parameter. In case of the pubkey parameter
the guest pointer to a 32 byte slice is taken instead.

### Changes to CU metering

CPI will no longer charge CUs for the length of account payloads. Instead TBD
CUs will be charged for every instruction account.

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

## Backwards Compatibility

The magic field (`u32`) and version field (`u32`) of ABI v2 are placed at the
beginning, where ABI v0 and v1 would otherwise indicate the number of
instruction accounts as an `u64`. Because the older ABIs will never serialize
more than a few hundred accounts, it is possible to differentiate the ABI
that way without breaking the older layouts.
