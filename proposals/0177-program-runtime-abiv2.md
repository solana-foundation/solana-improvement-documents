---
simd: '0177'
title: Program Runtime ABI v2
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Idea
created: 2024-10-01
feature: TBD
extends: SIMD-0184, SIMD-0185
---

## Summary

Align the layout of the virtual address space to large pages in order to avoid
account data copies while maintaining a simple address translation logic.

## Motivation

At the moment all validator implementations have to copy (and compare) data in
and out of the virtual memory of the virtual machine. There are four possible
account data copy paths:

- Serialization: Copy from program runtime (host) to virtual machine (guest)
- CPI call: Copy from virtual machine (guest) to program runtime (host)
- CPI return: Copy from program runtime (host) to virtual machine (guest)
- Deserialization: Copy from virtual machine (guest) to program runtime (host)

To avoid this a feature named "direct mapping" was designed which uses the
address translation logic of the virtual machine to emulate the serialization
and deserialization without actually performing copies.

Implementing direct mapping in the current ABI v0 and v1 was deemed too complex
because of unaligned virtual memory regions and memory accesses overlapping
multiple virtual memory regions. Instead the layout of the virtual address
space should be adjusted so that all virtual memory regions are aligned to
4 GiB.

## Alternatives Considered

What alternative designs were considered and what pros/cons does this feature
have relative to them?

## New Terminology

None.

## Detailed Design

SDKs will have to support both ABI v1 and v2 for a transition period. The
program runtime must only use ABI v2 if all programs in a transaction support
it. Programs signal their support through their SBPF version field (TBD) while
the program runtime signals which ABI is chosen through the serialized magic
field.

### The serialization interface

- Writing to readonly accounts fails the transaction, even if the exact same
data is written as already is there, thus even if no change occurs.
- The is-executable-flag is never set.
- The next rent collection epoch is not serialized.
- Readonly instruction accounts have no growth capacity.
- For writable instruction accounts additional capacity is allocated and mapped
for potential account growth. The maximum capacity is the length of the account
payload at the beginning of the transaction plus 10 KiB. CPI can not grow
beyond what the caller allowed as top-level instructions limit the potential
growth. Thus it makes sense to preallocate this capacity in the beginning of
the transaction when the writable accounts are copied in case the transaction
needs to be rolled back.

### The serialization layout

The following memory regions must be mapped into the virtual machine,
each starting at a 4 GiB boundary in virtual address space:

- Writable header:
  - Magic: `u32`: `0x76494241` ("ABIv" encoded in ASCII)
  - ABI version `u32`: `0x00000002`
  - Pointer to instruction data: `u64`
  - Length of instruction data: `u32`
  - Number of unique instruction accounts: `u16`
  - Number of instruction accounts: `u16`
  - Program key: `[u8; 32]`
  - For each unique instruction account:
    - Key: `[u8; 32]`
    - Owner: `[u8; 32]`
    - Flags: `u64` (bit 8 is signer, bit 16 is writable)
    - Lamports: `u64`
    - Pointer to account payload: `u64`
    - Account payload length: `u32`
    - Account payload capacity: `u32`
  - Instruction account index indirection for aliasing:
    - Index to unique instruction account: `u16`
- Readonly instruction data
- Writable payload of account #0
- Readonly payload of account #1
- Writable payload of account #2
- Writable payload of account #3
- ...

With this design a program SDK can (but no longer needs to) eagerly deserialize
all account metadata at the entrypoint. Because this layout is strictly aligned
and uses proper arrays, it is possible to directly calculate the offset of a
single accounts metadata with only one indirect lookup and no need to scan all
preceeding metadata. This allows a program SDK to offer a lazy interface which
only interacts with the account metadata fields which are needed, only of the
accounts which are of interest and only when necessary.

## Impact

This change is expected to drastically reduce the CU costs if all programs in
a transaction support it as the cost will no longer depend on the length of the
instruction account payloads or instruction data.

Otherwise, the change will be hidden in the SDK and thus be invisible to the
dApp developer.

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility

The magic field (`u32`) and version field (`u32`) of ABI v2 are placed at the
beginning, where ABI v0 and v1 would otherwise indicate the number of
instruction accounts as an `u64`. Because the older ABIs will never serialize
more than a few hundred accounts, it is possible to differentiate the ABI
that way without breaking the older layouts.
