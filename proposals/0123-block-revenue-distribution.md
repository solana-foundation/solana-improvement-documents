---
simd: '0123'
title: Block Revenue Sharing
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2024-03-10
feature: (fill in with feature tracking issues once accepted)
---

## Summary

A new mechanism is proposed to allow validators to share part of their block
revenue with their delegators. Commission rates from validator vote accounts
will be used by the protocol to calculate post-commission rewards that will be
automatically distributed to delegated stake accounts at the end of each epoch.

## Motivation

Delegated stake directly increases the number of blocks that a validator is
allocated in an epoch leader schedule but the core protocol doesn't support
diverting any of that extra revenue to stake delegators.

## Alternatives Considered

### Custom Fee Collector Account

In [SIMD-0232], an alternative approach was proposed to simply allow validators
to set a custom fee collector account. Any desired commission calculations could
be done in an onchain program which controls the received fee rewards held in
the custom fee collector account. The downside of this approach is that this
onchain program would need to be "cranked" periodically to move funds into the
validator's fee payer account.

[SIMD-0232]: https://github.com/solana-foundation/solana-improvement-documents/pull/232

### Distribute Rewards as Activated Stake

The runtime could ensure that any distributed stake rewards get activated as
well but it would require extra complexity in the protocol to support that
feature. Instead, stakers will receive inactive SOL in their stake accounts that
they will have to manage themselves. [SIMD-0022] aims to make this experience
better for stakers by allowing stake accounts to separately delegate any
unstaked balance in their accounts.

[SIMD-0022]: https://github.com/solana-foundation/solana-improvement-documents/pull/22

### Out of protocol reward distribution 

Due to the lack of core protocol support for distributing block revenue to
stakers, validators have developed their own solutions which are not enforced by
the core protocol. For example, the Cogent validator diverts part of its fee
revenue to NFT holders. But it's up the NFT holders to audit and hold Cogent
accountable to a specific commission rate.

Another alternative is Jito's mechanism for block "tips" (not fees, but the idea
is similar). Jito's validator implementation includes a tip distribution program
which it instructs validator operators to divert all of their tips to but cannot
enforce perfect compliance. It's up to stakers and the Jito team to audit
compliance by validator operators. This mechanism requires trusting a
third-party (in this case Jito) to calculate reward distribution in an accurate
and fair manner. It also relies on using a merkle tree to distribute fees to all
stake accounts and the distributed fees are not automatically staked in
recipient stake accounts.

## New Terminology

NA

## Detailed Design

### Block Revenue Collection

After all transactions are processed in a block for a given leader, rather than
collecting all block revenue into the validator identity account, the protocol
will check if the validator's vote account has specified a commission rate and
collector addresses in the new vote account version described in [SIMD-0185].
In order to eliminate the overhead of tracking the latest fee collector address
and commission of each vote account, the vote account state at the beginning of
the previous epoch MUST be used. This is the same vote account state used to
build the leader schedule for the current epoch.

If the commission rate and collector account aren't set, all revenue will be
collected into the validator's identity account as before. If the commission
rate and collector account *are* specified, the rewards MUST be distributed
according to the commission and delegator rewards collection sections below.

[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/pull/185

#### Commission Collection

The commission amount MUST be calculated by first multiplying the commission
rate by the amount of revenue and then using integer division to divide by
`10,000`. If the commission amount is non-zero, the fee collector account MUST
be loaded and checked for the following conditions:

1. account is system program owned AND
2. account is rent-exempt after depositing the commission.

If the conditions are met, the commission amount MUST be deposited into the fee
collector account. If either of these conditions is violated, the commission
amount MUST be burned.

#### Delegator Rewards Collection

The delegator rewards amount MUST be calculated by subtracting the calculated
commission from the block fee revenue. If the delegator rewards amount is
non-zero, the vote account must be loaded and checked for the following
conditions:

1. account is vote program owned AND
2. account is initialized with vote state v4 or later

If the conditions are met, the delegator rewards amount MUST be added to the
vote state field `pending_delegator_rewards` and added to the balance of vote
account. If either of these conditions is violated, the delegator rewards amount
MUST be burned.

### Delegator Rewards Distribution

After each epoch boundary (or after restart while epoch rewards are still
active), create a list of all vote accounts that existed and were initialized at
the beginning of the epoch. For each vote account, its state at the beginning of
the current epoch MUST be checked for its `pending_delegator_rewards` vote state
field, let's call this value `P`. If `P` is non-zero, record this value to
calculate individual delegator rewards as described below.

The amount of lamports distributed to an individual stake account can be
calculated by first summing all of the lamports that were actively staked during
the previous epoch for a given vote account, let's call this value `A`. Then,
the reward for an individual stake account can be calculated by multiplying its
active stake from the previous epoch by `P` and then integer dividing by `A`.
Fractional lamports will be discarded so not all `P` lamports for a given
delegator rewards pool will be distributed. After calculating all individual
stake rewards, sum them and call this value `D`. Then subtract (`P - D`) from
the vote account balance and the `pending_delegator_rewards` field and subtract
this value from the cluster capitalization.

After updating the vote account's `pending_delegator_rewards` field and
deducting any lamports that won't get distributed to stake delegators from the
vote account balance, store the vote account in accounts db before processing
any blocks in the new epoch.

#### Individual Delegator Reward

The stake reward distribution amounts for each stake account calculated above
can then be used to construct a list of stake reward entries which MUST be
partitioned and distributed according to [SIMD-0118].

When reward entries are used to distribute rewards pool funds during partitioned
rewards distribution, the delegated vote account for each rewarded stake account
must have its `pending_delegator_rewards` field and its balance deducted with
the amount of rewards distributed to keep capitalization consistent.

[SIMD-0118]: https://github.com/solana-foundation/solana-improvement-documents/pull/118

### Vote Program Changes

Since delegator rewards will be stored in the validator's vote account until
distribution at the next epoch boundary, those funds will be unable to be
withdrawn.

The `Withdraw` instruction will need to be modified so that if the balance
indicated by the `pending_delegator_rewards` field is non-zero, the vote account
will no longer be closeable by fully withdrawing funds. The withdrawable balance
when `pending_delegator_rewards` is non-zero will be equal to the vote account's
balance minus `pending_delegator_rewards` and the minimum rent exempt balance.

## Impact

Stake delegators will receive additional income when delegating to validators
who adopt this new feature.

## Security Considerations

NA

## Backwards Compatibility

A feature gate will be used to enable block reward collection and distribution
at an epoch boundary.
