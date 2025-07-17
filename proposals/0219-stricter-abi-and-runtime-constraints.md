---
simd: '0219'
title: Stricter ABI and Runtime Constraints
authors:
  - Sean Young
  - Alexander Meißner
category: Standard
type: Core
status: Review
created: 2025-01-06
feature: C37iaPi6VE4CZDueU1vL8y6pGp5i8amAbEsF31xzz723
---

## Summary

Removing pitfalls and foot-guns from the ABI (including syscalls) and runtime.

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
- VM memory access
  - Bad read accesses to account payload go unnoticed as long as they stay
  within the reserved address space, even if they leave the actual account
  payload
  - Bad write accesses to account payload go unnoticed as long as the original
  value is restored
- Syscall slice parameters
  - Bad read and write accesses which span nonsensical ranges go unnoticed

Furthermore, at the moment all validator implementations have to copy
(and compare) data in and out of the virtual memory of the virtual machine.
There are four possible account data copy paths:

- Serialization: Copy from program runtime (host) to virtual machine (guest)
- CPI call edge: Copy from virtual machine (guest) to program runtime (host)
- CPI return edge: Copy from program runtime (host) to virtual machine (guest)
- Deserialization: Copy from virtual machine (guest) to program runtime (host)

By restricting the allowed behavior of dApps we enable the validator to map
account payload data directly, avoiding copies and compares.

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

### VM memory access

Memory accesses (both by the program and by syscalls) which span across memory
mapping regions are considered access violations. Accesses to multiple regions
(e.g. by memcpy syscalls) have to be split into multiple separate accesses,
one for each region:

- Readonly data (`0x100000000..0x200000000`)
- Stack (`0x200000000..0x300000000`)
- Heap (`0x300000000..0x400000000`)
- Instruction meta data
- Account meta data
- Account payload address space
- Instruction payload

The payload address space of an account is the range in the serialized input
region (`0x400000000..0x500000000`) which covers the payload and optionally the
10 KiB resize padding (if not a loader-v1 program), but not the accounts
metadata.

For all memory accesses to the payload address space of an account which is
flagged as writable and owned by the currently executed program, check that:

- The access is completely within the maximum account length,
otherwise `InstructionError::InvalidRealloc` must be thrown.
- The access is completely within the rest of the account growth budget of the
transaction, otherwise `InstructionError::InvalidRealloc` must be thrown.
- The access is completely within the current length of the account,
otherwise extend the account with zeros to the maximum allowed by the previous
two checks.

For loads / read accesses to the payload address space of an account check
that:

- The access is completely within the current length of the account,
otherwise `InstructionError::AccountDataTooSmall` must be thrown.

For stores / write accesses to the payload address space of an account check
that:

- The account is flagged as writable,
otherwise `InstructionError::ReadonlyDataModified` must be thrown
- The account is owned by the currently executed program,
otherwise `InstructionError::ExternalAccountDataModified` must be thrown.

## Impact

These restrictions have been extensively tested by replay against MNB.
Most of the dApps devs whose dApps would fail have been contacted and had
their dApps fixed already.

Programs which used the SDKs account realloc function, which is now deprecated,
should upgrade in order to avoid the read-before-write access to uninitialized
memory.

## Security Considerations

None.
