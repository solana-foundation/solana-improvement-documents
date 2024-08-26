---
simd: '0148'
title: MoveStake and MoveLamports Instructions
authors:
  - Hanako Mumei
category: Standard
type: Core
status: Implemented
created: 2024-04-30
feature: [7bTK6Jis8Xpfrs8ZoUfiMDPazTcdPcTWheZFJTA5Z6X4](https://github.com/anza-xyz/agave/issues/1610)
development:
  - Anza - [implemented](https://github.com/anza-xyz/agave/pull/1415)
  - Firedancer - Implemented
---

## Summary

We propose introducing two new instructions to the stake program for moving
value between stake accounts with identical `Authorized` and `Lockup`:

* `MoveStake`: Move a given `amount` of active stake from one active account to
another active account, or from an active account to an inactive one, turning it
into an active account. If the entire source account delegation is moved, the
source account becomes inactive. In all cases, rent-exempt balance is unaffected
and minimum delegations are respected for accounts that end in an active state.
* `MoveLamports`: Move a given `amount` of excess lamports from one active or
inactive account to another active or inactive account, where "excess lamports"
refers to lamports that are neither delegated stake nor required for
rent-exemption.

For simplicity of implementation, we choose not to support accounts that are
activating, deactivating, or partially active. A future SIMD may choose to
extend this functionality should it be desirable.

## Motivation

Recently, a feature was activated which mandates that `Split` destinations be
prefunded with the rent-exempt reserve, because before that, `Split` could be
used to deactivate stake immediately, bypassing the cooldown period.

However, this has introduced issues for protocols that manage stake on behalf
of users without taking `Withdrawer` authority. Particularly, for one that
splits user stake across many validators and periodically redelegates between
them, every time they want to split part of a user stake to deactivate, the
protocol must fund the rent-exemption themselves. And then when that split
account is merged, those lamports cannot be reclaimed by the protocol, instead
accumulating (undelegated) in the destination merge accounts.

The purpose of the `MoveStake` instruction is to enable a flow whereby moving
stake from a user's stake accounts U1 -> U2 from validator V1 to validator V2
may proceed:

* `MoveStake` the `amount` of stake from the user stake account U1 to an
inactive account T holding sufficient lamports for rent exemption. T instantly
becomes a second active stake account delegated to V1 with `amount` stake.
* `Deactivate` T and wait an epoch.
* `DelegateStake` T to V2 and wait an epoch. T becomes an active stake account
delegated to V2 with `amount` stake.
* `MoveStake` the `amount` stake from T to U2. T returns to its initial inactive
state. Stake has moved from U1 to U2 with no outside lamports required and no
new undelegated lamports becoming trapped in delegated stake accounts.

The motivation for `MoveLamports` is to enable housekeeping tasks such as
reclaiming lamports from `Merge` destinations.

## Alternatives Considered

* There is a longstanding proposal we call Multistake, which would allow a
stake account to have two delegation amounts to the same validator, the idea
being that one serves as an onramp (and possibly offramp) for active stake on
the account. This could support other flows to accomplish the same objective,
such as allowing `DelegateStake` to accept an active stake account to begin
activating an account's excess (non-rent non-stake) lamports, or allowing a
`Split` source to `Deactivate` enough stake to cover rent-exemption for the new
account. However, Multistake is a much larger design/engineering project, and
we have to solve the existing proble sooner than Multistake would be ready.
* We discussed various proposals for allowing `Merge` to leave behind the
source account or `Split` to split into any mergeable destination. However this
confuses the presently clear distinction between these two operations and
entails additional implementation risk as they are already rather complex. A
new instruction that does one specific thing seems highly preferable.
* The original version of this SIMD proposed a `Move` that did not take an
`amount`, but this would require changes to `Split` to enable the first leg of
the proposed flow.
* Back out the changes introduced by requiring rent-exempt `Split` destinations.
This is undesirable because that restriction was added for very good reason: an
effectively unbounded amount of stake could be instantly deactivated through
repeated splitting.

## New Terminology

`MoveStake` and `MoveLamports`, two new stake program instructions.

## Detailed Design

For clarity of terminology inside this specification:

* An "active" stake is in a `Stake` state with nonzero delegation, 100% of which
is effective stake. There is no activating or deactivating stake.
* An "inactive" stake is in an `Initialized` or `Stake` state. There is no
effective, activating, or deactivating stake.

### `MoveStake`

`MoveStake` requires 3 accounts:

0. Source stake account: Writable, owned by the stake program
1. Destination stake account: Writable, owned by the stake program
2. Stake account authority: Read-only signer

`MoveStake` instruction data is 12 bytes, containing:

* `0x10 0x00 0x00 0x00`, a fixed-value four-byte little-endian unsigned integer
acting as the instruction discriminator
* `amount`, an unaligned eight-byte little-endian unsigned integer indicating
the quantity of lamports to move

`MoveStake` aborts the transaction when:

* `amount` is 0
* Source or destination are not writable
* Source or destination are not owned by the stake program
* Source and destination have the same address
* Source and destination do not have identical `Authorized`
* If `Lockup` is in force, source and destination do not have identical `Lockup`
* The stake account authority is not the `Staker` on both accounts
* The stake account authority is not a signer
* Source data length is not equal to the current version of `StakeState`
* Destination data length is not equal to the current version of `StakeState`
* Source is not active
* Destination is neither active nor inactive
* If destination is active, source and destination are not delegated to the same
vote account
* Moving `amount` stake would leave source with a nonzero amount of stake less
than the minimum delegation
* Moving `amount` stake would fail to bring destination up to minimum delegation

If all of these conditions hold, then:

* Delegation and lamports on source are debited `amount`
* Delegation and lamports on destination are credited `amount`
* If `amount` constitutes the full delegation on the source, source is reset to
an `Initialized` state
* If destination is inactive, destination becomes active with the same `Stake`
as source, aside from delegation amount
* `credits_observed` must be updated on the destination according to the same
rules as `Merge`

### `MoveLamports`

`MoveLamports` requires 3 accounts:

0. Source stake account: Writable, owned by the stake program
1. Destination stake account: Writable, owned by the stake program
2. Stake account authority: Read-only signer

`MoveLamports` instruction data is 12 bytes, containing:

* `0x11 0x00 0x00 0x00`, a fixed-value four-byte little-endian unsigned integer
acting as the instruction discriminator
* `amount`, an unaligned eight-byte little-endian unsigned integer indicating
the quantity of lamports to move

`MoveLamports` aborts the transaction when:

* `amount` is 0
* Source or destination are not writable
* Source or destination are not owned by the stake program
* Source and destination have the same address
* Source and destination do not have identical `Authorized`
* If `Lockup` is in force, source and destination do not have identical `Lockup`
* The stake account authority is not the `Staker` on both accounts
* The stake account authority is not a signer
* Source is neither active nor inactive
* Destination is not a valid merge destination (active, inactive, or activating
with zero effective stake)
* `amount` exceeds source `lamports - effective_stake - rent_exempt_reserve`

If all of these conditions hold, then:

* Lamports on source are debited `amount`
* Lamports on destination are credited `amount`

## Impact

The primary utility of the proposed instructions is to support protocol
developers in moving value between stake accounts with the same authorities
without controlling the `Withdrawer`. There is no loss of existing
functionality.

## Security Considerations

Care must be taken to ensure stakes are active, as moving delegations
between accounts in any kind of intermediate state is fraught. Otherwise this
change should be fairly low impact, as it does not require changing any existing
logic, in particular avoiding making `Split` or `Merge` more permissive.
