---
simd: '0490'
title: Upgrade BPF Stake Program to v5.0.0
authors:
  - Hanako Mumei
category: Standard
type: Core
status: Review
created: 2026-03-09
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD will upgrade the Core BPF Stake Program to v5.0.0.

## Motivation

The BPF Stake Program is under active development and requires feature-gated
releases to upgrade to new versions. This proposal is not in and of itself a new
feature.

Aside from the general desire to improve and maintain the BPF Stake Program,
this SIMD is necessary to actvate the
[Incremental Reduction of lamports_per_byte](https://github.com/solana-foundation/solana-improvement-documents/pull/437)
feature gates.

## New Terminology

N/A

## Detailed Design

BPF Stake v5.0.0 introduces several changes:

* Raise minimum delegation from 1 lamport to (XXX under discussion). This is a
medium-firm blocker on rent reduction because allowing the total lamports
required to open a stake account to fall is highly undesirable.
* Use the `Rent` sysvar in preference to `Meta.rent_exempt_reserve`. This is a
*hard* blocker on rent reduction to protect the integrity of mathematical
operations involving delegated stake. The value of `rent_exempt_reserve` for all
new stake accounts is held at `2_282_880`, the present rent-exempt reserve of a
200-byte stake account, to minimize downstream breakage.
* Make all sysvar account inputs optional. The Stake Program will continue to
gracefully accept existing instructions that include sysvars.
* Rewrite the implementation of `Split` to fix several longstanding bugs. Other
that removing self-split, the new `Split` processor remains backwards
compatible.

In validator clients this fetaure will use existing code to effect a Core BPF 
Program upgrade. Otherwise, the only required validator support is to return the
new minimum delegation for the `getMinimumDelegation` RPC call.

## Alternatives Considered

N/A

## Impact

* Dapps must handle minimum delegation properly, since it will now rise above 1
lamport. Ultimately the Stake Program is the arbiter of correctness and will
safely reject invalid state transitions.
* `Meta.rent_exempt_reserve` is now deprecated and dapps should calculate rent-
exemption via `Rent`. This is a consequence of rent reduction itself rather than
this SIMD specifically.
* Sysvars no longer need to be provided for Stake Program operations, reducing
transaction size and allowing CPI callers to also no longer require these
accounts in their own interfaces. Updated Stake Program instruction builders
will be released after BPF Stake v5.0.0 is live on all networks.
* The old version of `Split` treated initialized stakes and deactivated stakes
differently due to a longstanding bug with the side-effect that deactivated
stakes could sometimes not be `Split` from. This is now fixed and `Split` should
always work under the rules of its operation.
* When calling `Merge` on two activating stakes, only the source delegation and
`rent_exempt_reserve` would be merged into the destination delegation. We now
merge all lamports from the source account.

## Security Considerations

BPF Stake v5.0.0 will undergo security audit before deployment is allowed.

## Backwards Compatibility

Minimum delegation will rise above 1 lamport. Tooling or onchain programs that
assumes `Rent.minimum_balance(200) + 1` is sufficient to create a stake account
may break for very small balances.

Splitting a stake account into itself is now an error. There is no valid usecase
for such an act and as such we expect no breakage.
