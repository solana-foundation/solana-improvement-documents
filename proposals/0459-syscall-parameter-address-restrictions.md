---
simd: '0459'
title: Syscall Parameter Address Restrictions
authors:
  - Alexander Meißner (Anza)
category: Standard
type: Core
status: Review
created: 2026-01-30
feature: TBD
supersedes: 0219
---

## Summary

Syscall parameter related changes split off from SIMD-0219:
Removing pitfalls and foot-guns from the ABI (including syscalls) and runtime.

## Motivation

In a recent meeting between the Agave and Firedancer core developers it was
decided that SIMD-0219 should be split into two feature gates. This SIMD covers
the first half which is focused on reducing the attack surface and fix long
standing issues with CPI in general.

The reasoning for splitting SIMD-0219 is to de-risk its deployment and to be
able to make progress on shipping earlier parts even if later parts have to be
delayed further. The reason for cutting SIMD-0219 at this specific boundary is
that this first half is not expected to have any impact on performance, while
the second half on the other hand will.

Additionally, some consensus relevant code reordering is added to reduce the
attack surface of around CU metering and make the CPI logic more
comprehensible after this SIMDs feature gates are cleaned up.

## New Terminology

None.

## Detailed Design

### Changes inherited from SIMD-0219

- The following pointers must be on the stack or heap,
meaning their virtual address is inside `0x200000000..0x400000000`,
otherwise `SyscallError::InvalidPointer` must be thrown:
  - The destination address of all sysvar related syscalls
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

### Additional changes

`InstructionError::InvalidRealloc` must be thrown on the CPI call edge if the
caller requested an account length which does not fit the payload address space
of the account in the caller. This check must occur after the address of the
account length is translated but before the address of the account payload is
translated.

`InstructionError::InvalidRealloc` must be thrown on the CPI return edge if the
callee requested an account length which does not fit the payload address space
of the account in the caller. This check currently allows for +10 KiB even in
ABIv0 which is incorrect and must be reduced to 0 growth allowed in ABIv0.

All sites in the CPI code which perform guest to host address translation first
and then perform pointer arithmetic on the host must be swapped such that they
perform pointer arithmetic in the virtual address space first followed by the
address translation second. This specifically affects the account length field.

As a consequence of the changes to the address translation of the account
length field, the CU charging for the account length in CPI must be moved to
occur after the address translation of the account length field.

Modifications of the callee accounts on the CPI call edge must all happen
together after every account info and account meta is translated, instead of
being interleaved with these.

## Alternatives Considered

Leaving SIMD-0219 as is.

## Impact

Splitting SIMD-0219 should have no impact on dApp developers or validators.
The additional changes are continuously tested (over a month) and monitored
on mainnet-beta to not cause any existing dApps to behave differently.

## Security Considerations

None.
