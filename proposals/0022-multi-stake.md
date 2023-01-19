---
simd: '0022'
title: Multi Stake Account
authors:
  - Jon Cinque (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2023-01-20
feature: (fill in with feature tracking issues once accepted)
---

## Summary

A new "multi stake" account that allows for multiple movements of stake, useful
for redelegations, stake pools, and ordinary stakers.

## New Terminology

* "small stake": a stake whose delegation is below the minimum delegation
lamport amount, currently set to be 1 SOL
* "multi stake": a stake account with multiple `Delegation` instances

## Motivation

As a minimum stake delegation amount is applied to the network, and potentially
increased over time, it will become harder for small stakers to participate in
the network.

Current solutions through stake pools still carry some risk or capital inefficiency
for the stake pool operator. Either there's a requirement to leave liquid SOL available
for small stakers to exit, or small stakes cannot enter because there's not enough
to cover the minimum delegation amount.

Small stakes need to be delegated, while not causing problems in the validator's
stakes cache.

Side note: this isn't my first proposal for a new stake account type. Here's an earlier
attempt meant for redelegation: https://github.com/solana-labs/solana/pull/24762

## Alternatives Considered

The easiest solution is to change the validator's stakes cache, since that only
impacts the specific implementation in the Solana Labs validator.

In the perfect situation, small stakes are properly delegated, but don't receive
rewards. Meaning:

* they don't take up space in the stakes cache
* they take the correct amount of epoch boundaries to activate / deactivate
* they are included in a validator's voting power
* they can be slashed

There are a few ways of solving this at the stakes cache level, and they all have
serious issues.

* Include small stakes in the cache, but don't pay out rewards

This is the simplest solution, which essentially changes "minimum delegation amount"
to mean "minimum reward earning amount". So you can delegate stake accounts with
less than the minimum delegation, but they won't earn rewards.

Unfortunately, small stakes can still bloat the stakes cache from a memory level.

* Don't include small stakes in the cache, but track them in transactions

In this solution, on every transaction, if the runtime sees a small stake, it
includes its delegation amount in the validator's voting power, but doesn't
add its pubkey and `Delegation` to the cache.

The runtime tracks the pre and post state of every account, and for a successful
transaction, subtracts the old small stake delegation amount, and adds the new small
stake delegation amount.

During rewards payout, small stakes are completely omitted, since they are not
present in the cache.

During a slashing event, it's still possible to query it using `get_program_accounts`.

This is incredibly brittle, however, and introduces more overhead in the runtime.
The bank must hold onto pre-states for all transactions to debit the stakes cache. And,
if called from the outside, all `store_accounts` functions on the bank could easily
invalidate the stakes cache.

For example, if you store a small stake directly, what does the cache do? What
was the pre-state of that small stake? There's no way to know.

## Detailed Design

Rather than hacking the validator, let's change the stake program.

With a new "multi stake" account, carrying *two* more `Delegation`s, the stake
program becomes much more flexible.

During initial delegation, the "multi stake" account must have enough to cover
the minimum stake delegation amount. The two other `Delegation` instances may be
used for small stake movements.

Here are some example uses:

### Add or remove small stake

To add a small stake to your account, transfer the lamports, call `delegate` once
more, and now your stake account has two delegations: the initial one, and the
newly activating one.

Once the new `Delegation` is active, a permissionless `update` instruction follows
the existing `merge` logic to consolidate the two `Delegation`s into one.

The same roughly applies for removing a small stake. You `deactivate` a portion
of it, which adds another delegation to your stake account: the initial one, and
the newly deactivating one.

### Redelegate

With another `Delegation` instance, you can redelegate from one validator to another
within the same account.

It works the same as the existing "redelegate" instruction, except the lamports
all stay in the same account, and the second `Delegation` covers the redelegation.

The permissionless `update` instruction clears out the first one once it's inactive.

### Single validator stake pool

A single-validator stake pool program can manage small stakes with 100% efficiency
using "multi stakes".

Since there's a total of *three* `Delegation` instances, small stakes can be added
and removed at the same time. One instance covers the main amount, another covers
new activations, and the last one covers deactivations.

For small stake deposit, you simply transfer the lamports and activate the new amount.

For small stake withdrawal, the program deactivates the small stake and provides
a claimable ticket to the user, used to claim their lamports after deactivation.

### Runtime

The main difference to the runtime is processing up to three `Delegation`s
per account.

There's minor impact on the stakes cache, which gets a little bigger, but the
maximum is bounded by the minimum delegation amount.

There's minor impact on rewards payout, since the time spent calculating the rewards
amount is negligible compared to the time spent storing the accounts. The number
of account storages stays the same.

## Impact

Validators likely see a small increase in memory usage from additional `Delegation`
entries in the stakes cache, and perhaps a tiny increase in rewards payout time,
but nothing substantial.

Dapp developers have to deal with a new stake account type. This may break programs
that try to deserialize any stake account.

The network benefits as a whole:

* stake accounts are more flexible for adding or removing stake, without splits and merges
* stakers can easily redelegate without losing funds
* single-validator stake pools can be just about 100% capital efficient

## Security Considerations

It's risky to change the runtime, especially the stake program and rewards payout.
Thankfully, since this all operates on the level of the `Delegation`, as long
as we pass all the instances as needed, none of the most sensitive parts need
to change.

## Drawbacks *(Optional)*

We could just change the stakes cache include small stakes, but not in rewards
payout, and everything mostly works, but it's ultimately a temporary solution.

## Backwards Compatibility *(Optional)*

While existing programs that create and use their own stake accounts are not impacted,
those that accept any stake account will crash on multi-stakes. There's no way
around that, since the proposal entails a new stake account variant.
