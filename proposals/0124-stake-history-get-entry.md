---
simd: '0124'
title: Get Stake History Entry
authors:
  - Hana Mumei
category: Standard
type: Core
status: Draft
created: 2024-03-14
feature: (fill in with feature tracking issues once accepted)
---

## Summary

We propose a new syscall, `GetStakeHistoryEntry` which accepts an `Epoch` and
uses `StakeHistory` from the `SysvarCache` to return the `StakeHistoryEntry` for
that epoch.

## Motivation

We are in the process of porting the stake program from its current native form
to a bpf program independent of the validator, as part of the work to support
Firedancer. We have two reasons to add a syscall of this form:

* `StakeHistory` is 16kb, four times the size of the stack and fully half the
size of the heap. Deserializing this struct would be onerous for a bpf program,
especially since the stake program is one of the most frequently used by cpi.
* More importantly, due to the `require_rent_exempt_split_destination` feature,
`Split` requires `StakeHistory` but does not pass it in the account list. We
cannot port the stake program to bpf without an out-of-band way of accessing
`StakeHistory::get()` without breaking interface compatability.

As an aside, the current design of the program assumes that `Redelegate` will
not be activated and does not implement it. But if we do end up activating it,
we will need this syscall for `Delegate`, `Deactivate`, and
`DeactivateDelinquent` as well.

## Alternatives Considered

* The deserialization size cost could be addressed by searching the unparsed
`[u8]` data instead of deserializing it. This however does not address the
`Split` issue.
* We could implement `Sysvar::get()` for `StakeHistory` in its entirety similar
to `Clock` and `Rent`. This however seems highly undesirable because it does
not address the size cost, and we also will never need more than one entry
unless greater than 9% of stake goes into warmup cooldown.
* We investigated the possibility of making something like `SysvarCache`
available to programs, leaning on the existing parsed structs in host data.
However there is presently no way to access these from a vm without copying the
data, so this does not address the size cost. Furthermore, we would need a
mechanism for programs to signal what sysvars they need access to, because
imposing the cost of copying `StakeHistory` and `SlotHashes` on all programs is
unacceptable.
* We could break the interface. This is a non-starter in my opinion however.
* We could disable `require_rent_exempt_split_destination`. This does not seem
desirable.

## New Terminology

(none)

## Detailed Design

A reference implementation is provided at [agave
212](https://github.com/anza-xyz/agave/pull/212), but it is virtually identical
to the syscalls that underlie `Sysvar::get()` for `Clock`,`Rent`, etc.

The syscall accepts a pointer to `Option<StakeHistoryEntry>` plus an `Epoch`. It
charges compute units equivalent to `sysvar_base_cost` plus
`size_of::<Option<StakeHistoryEntry>>()`, gets `StakeHistory` from cache, and
uses a function equivalent to `StakeHistory::get()` to binary search the history
for the entry for that epoch, which it then writes to memory.

## Impact

The only impact I can foresee is that Firedancer would need to implement it, but
the people on their end we've been in contact with are in support since it moves
forward the native to bpf project. This is the only syscall we expect to need to
complete the stake program conversion. A similar syscall will likely be required
for `SlotHashes` for the vote program.

## Security Considerations

(none)

## Backwards Compatibility

This simd does not impact backwards compatibility. It is worth noting, however,
that it does not lock us into supporting it forever either. When direct mapping
is available, we can deprecate this and all other `Sysvar::get()`-like syscalls
in favor of a unified `SysvarCache` approach.
