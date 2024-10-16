---
simd: '0185'
title: Stricter VM verification constraints
authors:
  - Sean Young
  - Alexander Meißner
category: Standard
type: Core
status: Idea
created: 2024-10-16
feature: TBD
---

## Summary

Removing pitfalls and foot-guns from the virtual machine.

## Motivation

There are a couple of interactions between dApps and the virtual machine which
are currently allowed but make no sense and are even dangerous for dApps to
go unnoticed.

TODO:

- CPI verification
  - Allows accidentally using `AccountInfo` structures which the program
  runtime never serialized
  - `AccountInfo` structures can be overwritten by CPI during CPI, causing
  complex side effects
- VM stack gaps
  - Complicates virtual address calculations
  - False sense of security,
  dApps which overrun their stack go unnoticed anyway
  - Unaligned accesses near the edge of a stack frame can bleed into the next
- VM write access
  - Bad write accesses go unnoticed if another error corrects them
- Syscall virtual address ranges
  - Bad read and write accesses go unnoticed

## Alternatives Considered

None.

## New Terminology

None.

## Detailed Design

### CPI verification

- The special treatment during CPI of instruction accounts with the
`is_executable` flag set is removed
- The following pointers must be on the stack or heap,
meaning their virtual address is inside `0x200000000..0x400000000`,
otherwise `SyscallError::InvalidPointer` must be thrown:
  - The pointer in the array of `&[AccountInfo]` / `SolAccountInfo*`
  - The `AccountInfo::data` field,
  which is a `RefCell<&[u8]>` in `sol_invoke_signed_rust`
- The following pointers must point to what was originally serialized in the
input regions by the program runtime,
otherwise `SyscallError::InvalidPointer` must be thrown:
  - `AccountInfo::key` / `SolAccountInfo::key`
  - `AccountInfo::owner` / `SolAccountInfo::owner`
  - `AccountInfo::lamports` / `SolAccountInfo::lamports`
  - `AccountInfo::data::ptr` / `SolAccountInfo::data`

### VM stack

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

### Syscall virtual address ranges

When a range in virtual address space which:

- crosses a 4 GiB aligned boundary
- starts in any accounts payload (including its resize padding) and leaves it

is passed to any syscall (such as `memcpy`) it must throw
`SyscallError::InvalidLength`.

## Impact

These restrictions have been extensively tested by replay against MNB.
Most of the dApps devs whose dApps would fail have been contacted and had
their dApps fixed already.

## Security Considerations

None.
