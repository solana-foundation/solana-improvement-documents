---
simd: '0148'
title: MoveStake and MoveLamports Instructions
authors:
  - Hanako Mumei
category: Standard
type: Core
status: Draft
created: 2024-04-30
feature: (fill in with feature tracking issues once accepted)
---

## Summary

We propose introducing two new instructions to the stake program for moving
value between stake accounts with identical `Authorized` and `Lockup`:

* `MoveStake`: Move a given `amount` of active stake from one fully active
account to another fully active account, or from a fully active account to an
inactive one, turning it into an active account. In all cases, rent-exempt
balance is unaffected and minimum delegations are respected.
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

* `MoveStake` the `amount` of stake from the user stake account U1 to a
"transient" inactive account T holding sufficient lamports for rent exemption
and minimum delegation. T instantly becomes a second active stake account
delegated to V1 with `amount` stake.
* `Deactivate` T and wait an epoch.
* `DelegateStake` T to V2 and wait an epoch. T becomes an active stake account
delegated to V2 with `amount + minimum_delegation` stake.
* `MoveStake` the `amount` stake from T to U2. `Deactivate` T to return it to
its initial state. Stake has moved from U1 to U2 with no outside lamports
required and no new undelegated lamports in delegated stake accounts.

The motivation for `MoveLamports` is to enable housekeeping tasks such as
reclaiming lmaports from `Merge` destinations.

## Alternatives Considered

* There is a longstanding proposal we call Multistake, which would allow a
stake account to have two delegation amounts to the same validator, the idea
being that one serves as an onramp (and possibly offramp) for active stake on
the account. This could support other flows to accomplish the same objective,
such as allowing `DelegateStake` to accept an active stake account to begin
activating an account's excess (non-rent non-stake) lamports, or allowing a
`Split` source to `Deactivate` enough stake to cover rent-exemption for the new
account. However, Multistake is a much larger design/engineering project, and
we have to solve this sooner than it would be ready.
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

### `MoveStake`

`MoveStake` requires 5 accounts:

* Source stake account
* Destination stake account
* Clock
* Stake history
* Stake account authority

`MoveStake` requires 1 argument:

* `amount`, a `u64` indicating the quantity of lamports to move

`MoveStake` aborts the transaction when:

* `amount` is 0
* Source and destination have the same address
* Source and destination do not have identical `Authorized` and `Lockup`
* The stake account authority is not the `Staker` on both accounts
* The stake account authority is not a signer
* Destination data length is not equal to the current version of `StakeState`
* Source is not fully active (100% of delegation is effective)
* Destination is neither fully active nor fully inactive (initialized or
deactivated)
* If destination is active, source and destination are not delegated to the same
vote account
* Moving `amount` stake would bring source below minimum delegation
* Moving `amount` stake would fail to bring destination up to minimum delegation

If all of these conditions hold, then:

* Delegation and lamports on source are debited `amount`
* Delegation and lamports on destination are credited `amount`
* If destination is inactive, it is set to active with the same `Stake` as
source, aside from delegation amount

### `MoveLamports`

Accounts and arguments are identical to the above.

`MoveLamports` aborts the transaction when:

* `amount` is 0
* Source and destination have the same address
* Source and destination do not have identical `Authorized` and `Lockup`
* The stake account authority is not the `Staker` on both accounts
* The stake account authority is not a signer
* Source is neither fully active nor fully inactive
* Destination is neither fully active nor fully inactive
* `amount` exceeds source `lamports - stake - rent_exempt_reserve`

If all of these conditions hold, then:

* Lamports on source are debited `amount`
* Lamports on destination are credited `amount`

## Impact

The primary utility of the proposed instructions is to support protocol
developers in moving stake without controlling the `Withdrawer`. There is no
loss of existing functionality.

## Security Considerations

Care must be taken to ensure stakes are fully active, as moving delegations
between accounts in any kind of transient state is fraught. Otherwise this
change should be fairly low impact, as it does not require changing any existing
logic, in particular avoiding making `Split` or `Merge` more permissive.
