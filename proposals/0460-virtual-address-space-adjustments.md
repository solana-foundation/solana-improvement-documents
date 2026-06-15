---
simd: '0460'
title: Virtual Address Space Adjustments
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

Virtual address space related changes split off from SIMD-0219:
Removing pitfalls and foot-guns from the ABI (including syscalls) and runtime.

## Motivation

In a recent meeting between the Agave and Firedancer core developers it was
decided that SIMD-0219 should be split into two feature gates. This SIMD covers
the second half which is focused on the address translation, memory mappings,
reallocation / resizing, zeroing and defining empty / unmapped address space.

The reasoning for splitting SIMD-0219 is to de-risk its deployment and to be
able to make progress on shipping earlier parts even if later parts have to be
delayed further.

Additionally, as the unaligned address translation mechanism required for this
part of SIMD-0219 incurs a slight performance decrease, it was decided to
globally deactivate stack frame gaps to counter the effect. Stack frame gaps
were identified as a performance issue in the address translation in general.

## New Terminology

None.

## Detailed Design

### Changes inherited from SIMD-0219

Memory accesses (both by the program and by syscalls) which span across memory
mapping regions are considered access violations. Accesses to multiple regions
(e.g. by memcpy syscalls) have to be split into multiple separate accesses
(one for each region) by the user.

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

### Additional changes

Stack frame gaps must be deactivated globally (even for existing SBPFv0
programs). So far they were only deactivated for newer SBPF versions.

After the removal of the gaps the stack address space will be compacted by a
factor of two but the total mapped bytes will stay the same. They will simply
be at lower addresses than before and form one contiguous address range. See
example below:

| Address range | Before this SIMD | After this SIMD |
| --- | --- | --- |
| `0x200000000..0x200001000` | mapped | mapped |
| `0x200001000..0x200002000` | --- | mapped |
| `0x200002000..0x200003000` | mapped | mapped |
| `0x200003000..0x200004000` | --- | --- |
| `0x200004000..0x200006000` | mapped | --- |
| `0x200005000..0x200007000` | --- | --- |

In SBPFv0 the stack frame bump on `call` and `callx` must be lowered from 8 KiB
to 4 KiB (this is already the case in SBPFv3).

## Alternatives Considered

Leaving SIMD-0219 as is.

## Impact

Splitting SIMD-0219 should have no impact on dApp developers or validators.
An experiment to gauge the impact of the additional changes is currently
running.

## Security Considerations

Removing the stack frame gaps globally (even for SBPFv0) programs can affect
programs which rely on the address space layout of the stack either by using
absolute addressing, relative addressing across stack frames or being dependent
on bugs such accesses reaching over the boundary of stack frames.
