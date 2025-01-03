---
simd: '0219'
title: Stricter VM verification constraints
authors:
  - Sean Young
  - Alexander Meißner
category: Standard
type: Core
status: Review
created: 2025-01-06
feature: GJVDwRkUPNdk9QaK4VsU4g1N41QNxhy1hevjf8kz45Mq
---

## Summary

Removing pitfalls and foot-guns from the virtual machine and syscalls.

## Motivation

There are a couple of interactions between dApps and the virtual machine which
are currently allowed but make no sense and are even dangerous for dApps:

- CPI verification
  - Allows accidentally using `AccountInfo` structures which the program
  runtime never serialized
  - `AccountInfo` structures can be overwritten by CPI during CPI, causing
  complex side effects
- Gaps in between VM stack frames
  - Complicates virtual address calculations
  - False sense of security, dApps which overrun their stack can go unnoticed
  anyway if they overrun it by an entire frame
  - Unaligned accesses near the edge of a stack frame can bleed into the next
- VM write access
  - Bad write accesses to account payload go unnoticed as long as the original
  value is restored
- Syscall slice parameters
  - Bad read and write accesses which span nonsensical ranges go unnoticed

## Alternatives Considered

None.

## New Terminology

None.

## Detailed Design

### CPI verification

- The following pointers must be on the stack or heap,
meaning their virtual address is inside `0x200000000..0x400000000`,
otherwise `SyscallError::InvalidPointer` must be thrown:
  - The pointer in the array of `&[AccountInfo]` / `SolAccountInfo*`
  - The `AccountInfo::data` field,
  which is a `RefCell<&[u8]>` in `sol_invoke_signed_rust`
  - The `AccountInfo::lamports` field,
  which is a `RefCell<&u64>` in `sol_invoke_signed_rust`
- The following pointers must point to what was originally serialized in the
input regions by the program runtime,
otherwise `SyscallError::InvalidPointer` must be thrown:
  - `AccountInfo::key` / `SolAccountInfo::key`
  - `AccountInfo::owner` / `SolAccountInfo::owner`
  - `AccountInfo::lamports` / `SolAccountInfo::lamports`
  - `AccountInfo::data::ptr` / `SolAccountInfo::data`

### Gaps in between VM stack frames

The virtual address space of the stack frames must become consecutive:

- From: `0x200000000..0x200001000`, `0x200002000..0x200003000`, ...
- To: `0x200000000..0x200001000`, `0x200001000..0x200002000`, ...

This goes for all programs globally and is not opt-in.
Thus, this change is independent of SIMD-0166.

### VM write access

When a write access to the input region (`0x400000000..0x500000000`) happens,
which overlaps with a range in which an accounts payload, including its resize
padding but not its metadata, was serialized it must be checked that:

- The account is flagged as writable,
otherwise `InstructionError::ReadonlyDataModified` must be thrown
- The account is owned by the currently executed program,
otherwise `InstructionError::ExternalAccountDataModified` must be thrown

Thus, changing and later restoring data in unowned accounts is prohibited.

### Syscall slice parameters

When a range in virtual address space which:

- starts in any account data (including its resize padding) and leaves it
- starts outside account data and enters it

is passed to `memcpy`, `memmove`, `memset`, or `memcmp`, it must throw
`SyscallError::InvalidLength`.

Except for CPI, all other syscalls which
act on ranges in the virtual address space are confined to a single
memory region for now. Meaning they have to stay within one of:

- Readonly data
- Stack
- Heap
- Account meta data
- Account data without resize padding
- Account resize padding

And can not cross into any other region. This restriction is planned to
be lifted in another SIMD.

## Impact

These restrictions have been extensively tested by replay against MNB.
Most of the dApps devs whose dApps would fail have been contacted and had
their dApps fixed already.

## Security Considerations

None.
