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
automatically distributed to delegated stake accounts at the end of each epoch
via delegator reward pools.

## Motivation

Delegated stake directly increases the number of blocks that a validator is
allocated in an epoch leader schedule but the core protocol doesn't support
diverting any of that extra revenue to stake delegators.

## Alternatives Considered

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

Delegator Rewards Pool Account: An account that pools rewards that will be
distributed to the stake delegators for a particular validator at the end of an
epoch.

## Detailed Design

### Block Revenue Collection

After all transactions are processed in a block for a given leader, rather than
collecting all block revenue into the validator identity account, the protocol
will check if the validator's vote account has specified a commission rate and
collector addresses in the new vote account version described in
[SIMD-0185](https://github.com/solana-foundation/solana-improvement-documents/pull/185).
If the commission rate and collector account aren't set, all revenue will be
collected into the validator's identity account as before. If the commission
rate and collector account *are* specified, the calculated commission MUST be
deposited in the collector account and the remaining rewards MUST be deposited
into the delegator rewards pool account described in
[SIMD-0185](https://github.com/solana-foundation/solana-improvement-documents/pull/185)
as well.

#### Commission Collection

The commission amount MUST be calculated by first multiplying the rate by the
amount of revenue and then using integer division to divide by `10,000`. If the
commission amount is non-zero, the fee collector account MUST be loaded and
checked for the following conditions:
1. account is system program owned AND
2. account is rent-exempt after depositing the commission.

If the conditions are met, the commission amount MUST be deposited into the fee
collector account. If either of these conditions is violated, the commission
amount MUST be burned.

#### Delegator Rewards Pool Collection

The delegator rewards amount MUST be calculated by subtracting the calculated
commission from the block fee revenue. If the delegator rewards amount is
non-zero, a delegator rewards pool account address MUST be derived from the
block producer's vote account.

```rust
let delegator_rewards_pool_address = Pubkey::create_program_address(
    [
        b"delegator_rewards_pool",
        block_producer_vote_pubkey.as_ref(),
        &[block_producer_vote_account.delegator_rewards_pool_bump_seed],
    ],
    &stake_program::id(),
);
```

Then the delegator rewards pool account MUST be loaded and checked for the
following conditions:
1. account is stake program owned AND
2. account is initialized as `StakeState::DelegatorRewardsPool` AND
3. account is rent-exempt after depositing the reward.

If the conditions are met, the reward amount MUST be deposited into the
delegator rewards pool account. If any of these conditions is violated, the
reward amount MUST be burned.

### Delegator Rewards Pool Distribution

At the beginning of an epoch, for each unique vote account in the previous
epoch's leader schedule, the protocol REQUIRES checking if a derived stake
rewards pool account exists, is initialized, and has a lamport surplus above its
rent-exempt balance. For every delegator rewards pool with a lamport surplus,
the lamport surplus is considered to be activating stake.

#### Epoch Stake Activation Limit

Only 9% of the previous epoch's total active stake can be activated in a new
epoch. In order to prevent opening a loophole to circumvent that limit,
delegator rewards pool distributions will also be limited such that the sum of
all new stake activations and all delegator rewards pool distributions cannot
exceed 9% of the previous epoch's total active stake.

To calculate the amount of rewards to be distributed for a given delegator
rewards pool account, the protocol must first compute the sum of all stake
pending activation and all stake pending distribution from rewards pools, let's
call this value `P`. Then the protocol should multiply the previous epoch's
effective stake by 9 and then divide by 100 using integer division to get the
maximum amount of stake that can be activated this epoch, let's call this value
`M`. If `P > M`, then not all pending stake will get activated this epoch. For
each delegator rewards pool, if `P <= M` then entire lamport surplus will be
distributed, but if `P > M` the amount of stake to be distributed will be
calculated by multiplying the lamport surplus by `P` and then integer dividing
by `M`. Let's call the distribution amount for a given delegator rewards pool
account `d`.

#### Individual Delegator Reward

The amount of lamports distributed to an individual stake account can be
calculated by first summing all of the lamports that were actively staked during
the previous epoch for a given vote account, let's call this value `a`. Then,
the reward for an individual stake account can be calculated by multiplying the
active stake for an individual stake account by `d` and then integer dividing by
`a`. Fractional lamports will be discarded so not all `d` lamports for a given
delegator rewards pool will be distributed. After calculating all individual
stake rewards, the remaining lamports from `d` should be subtracted from the
stake rewards pool balance and burned.

These stake reward distribution amounts for each stake account will be included
into a list of reward entries which MUST be partitioned and distributed
according to
[SIMD-0118](https://github.com/solana-foundation/solana-improvement-documents/pull/118).

When reward entries are used to distribute rewards pool funds during partitioned
rewards distribution, the delegator rewards pool account balance must be
deducted with the amount of rewards distributed to keep capitalization
consistent.

## Impact

Stake delegators will receive additional income when delegating to validators
who adopt this new feature.

## Security Considerations

NA

## Backwards Compatibility

A feature gate will be used to enable block reward collection and distribution
at an epoch boundary.
