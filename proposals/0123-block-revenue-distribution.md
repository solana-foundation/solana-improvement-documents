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

A new mechanism for is proposed to allow validators to share part of their block
revenue with their delegators. Commission rates from validator vote accounts
will be used by the protocol to calculate post-commission rewards that will be
automatically distributed to delegated stake accounts at the end of each epoch.

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
and fair manner. It also relies on using a merkle tree to distribute fees to
all stake accounts and the distributed fees are not automatically staked in
recipient stake accounts.

### Partitioned reward calculation

The proposed design requires that all stake rewards pool accounts are updated
before processing any transactions in the first block of an epoch. This adds
more computation overhead at epoch boundaries but this overhead is directly
proportional to the number of vote accounts which are similarly updated at the
beginning of each epoch when calculating stake inflation rewards. If this
calculation overhead becomes an issue, the reward bookkeeping in stake rewards
pool accounts will need to be modified so that rewards for the new epoch do not
interfere will reward calculations for the previous epoch.

## New Terminology

Stake Rewards Pool Account: An account that records how many rewards from block
revenue earned by a particular validator will be distributed to stake delegators
at the end of an epoch.

## Detailed Design

### Block Revenue Collection

After all transactions are processed in a block for a given leader, rather than
collecting all block revenue into the validator identity account, the protocol
will check if the validator's vote account has specified commission rates and
collector addresses in the new vote account version described in SIMD-XXXX.
Block revenue could be composed of multiple distinct revenue sources such as
block fees and block tips which are configured by validators independently but
for the sake of simplifying this proposal, it's assumed that there will only be
a single revenue source for now: block fees, which includes both base signature
fees and priority fees.

If the commission rate and collector account for a revenue source aren't set,
all revenue will be collected into the validator's identity account as before.
If the commission rate and collector account for a revenue source *are*
specified, the commission amount MUST be calculated by first multiplying the
rate by the amount of revenue and then using integer division to divide by one
hundred. Then the commission amount should be deposited into the specified
collector address ONLY if the collector account is system program owned and
would be rent-exempt after the deposit.

For each revenue source, the amount of rewards that will be distributed to stake
delegators is calculated by subtracting the commission from the block revenue.
This amount will be deposited in a stake rewards pool account derived from the
block producer's vote address. Note that this differs from how post-commission
inflation rewards are calculated because no lamports are burned during the
commission split. In contrast, post-commission inflation rewards are calculated
by multiplying the amount of inflation rewards by `(100 - commission rate)` and
then using integer division to divide by hundred. Since both commission and
post-commission round down fractional lamports via integer division, any
leftover lamports are effectively burned.

If the reward distribution amount is non-zero, a stake rewards pool account
address should be derived from the validator's vote account and loaded from
accounts db. The address MUST be derived with the following derivation seeds for
the stake program:

```rust
let rewards_pool_address = Pubkey::create_program_address(
    [
        b"stake_rewards_pool",
        vote_pubkey.as_ref(),
        &[vote_account.rewards_pool_bump_seed],
    ],
    &stake_program::id(),
);
```

If a derived stake rewards pool account doesn't already exist or has not been
initialized, it should be created with the stake program as its owner and a data
size of 4 bytes. Its balance should be initialized with a rent exempt balance
which will increase total cluster capitalization similar to how sysvars are
created. Its data should be initialized with the `StakeState::RewardsPool` enum
variant discriminant `3u32` little endian encoded.

```rust
pub enum StakeState {
    Uninitialized,
    Initialized(Meta),
    Stake(Meta, Stake, StakeFlags),
    RewardsPool,
}
```

Once the derived stake rewards pool account exists and is initialized along with
a rent exempt balance, its balance should be increased by the amount of
post-commission revenue for each revenue source.

### Stake Rewards Pool Distribution

At the beginning of an epoch, for each unique vote account in the previous
epoch's leader schedule, the protocol REQUIRES checking if a derived stake
rewards pool account exists, is initialized, and has a lamport surplus above its
rent-exempt balance. For every stake rewards pool with a lamport surplus, the
lamport surplus is considered to be activating stake.

#### Epoch Stake Activation Limit

Only 9% of the previous epoch's total active stake can be activated in a new
epoch. In order to prevent opening a loophole to circumvent that limit, stake
rewards pool distributions will also be limited such that the sum of all new
stake activations and all stake rewards pool distributions cannot exceed 9% of
the previous epoch's total active stake.

To calculate the amount of stake rewards to be distributed for a given stake
rewards pool account, the protocol must first compute the sum of all stake
pending activation and all stake pending distribution from rewards pools, let's
call this value `P`. Then the protocol should multiply the previous epoch's
effective stake by 9 and integer divided by 100 to get the maximum amount of
stake that can be activated this epoch, let's call this value `M`. If `P > M`,
then not all pending stake will get activated this epoch. For each stake rewards
pool, if `P <= M` then entire lamport surplus will be distributed, but if `P >
M` the amount of stake to be distributed will be calculated by multiplying the
lamport surplus by `P` and then integer dividing by `M`. Let's call the
distribution amount for a given stake rewards pool account `d`.

#### Individual Stake Reward

The amount of lamports distributed to an individual stake reward can be
calculated by first summing all of the lamports that were actively staked during
the previous epoch for a given vote account, let's call this value `a`. Then,
the reward for an invididual stake account can be calculated by multiplying the
active stake for an individual stake account by `d` and then integer dividing by
`a`. Fractional lamports will be discarded so not all `d` lamports for a given
stake rewards pool will be distributed. After calculating all individual stake
rewards, the remaining lamports from `d` should be subtracted from the stake
rewards pool balance and burned.

These stake reward distribution amounts for each stake account will be included
into a list of reward entries which MUST be partitioned and distributed
according to SIMD-0118.

When reward entries are used to distribute rewards pool funds during paritioned
rewards distribution, the stake rewards pool account balance must be deducted
with the amount of rewards distributed to keep capitalization consistent.

## Impact

Stake delegators will receive additional stake reward income when delegating to
validators who adopt this new feature.

## Security Considerations

NA

## Backwards Compatibility

A feature gate will be used to enable block reward collection and distribution
at an epoch boundary.