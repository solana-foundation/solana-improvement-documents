---
simd: '0185'
title: Vote Account v4
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2024-10-17
feature: (fill in with feature tracking issues once accepted)
---

## Summary

A new update for the vote program is proposed to improve authorized voter
bookkeeping as well as allow validators to set commission rates and collector
accounts for different revenue sources.

## Motivation

This SIMD details two different modifications to the vote program and vote
account state and combines these changes into one proposal so that only
one vote account version update is needed.

### Revenue Collection Customization

- There is only one commission rate stored in vote account state but validators
want to be able to use different commission rates for different income streams
like block rewards.

- It's not possible to customize which accounts income is collected into.
Currently all block rewards are collected into the validator identity account
which cannot be a cold wallet since the identity needs to sign a lot of messages
for various network protocols used in Solana like turbine, gossip, and QUIC.

### Authorized voter bookkeeping

- Over 40% of vote state size is reserved for tracking a history of 32 prior
authorized voters for the vote account in the `prior_voters` field. Having such
a long history of prior voters is arguably not very useful, tracking the most
recent previous epoch's voter is probably sufficient and can be stored in the
`authorized_voters` field instead.

- The `authorized_voters` field doesn't store the voter for the previous epoch
so it's impossible to have a transition epoch where both the previous and newly
assigned voter can both sign votes.

## Alternatives Considered

### Reuse vote commission

Vote accounts already allow validators to set a commission rate for inflation
rewards and so it's not unreasonable to expect that this commission rate could
also be used to distribute block rewards. However, some validators have
expressed a desire to be able to tune revenue streams indpendently.

## New Terminology

- Block Rewards Collector: The account used to collect commissioned block
rewards for validators. Previously collected by default into the validator
identity account.

- Inflation Rewards Collector: The account used to collect commissioned
inflation rewards for validators. Previously collected by default into the vote
account.

- Block Rewards Commission: The commission rate that determines how much of
block base fee and priority fee revenue is collected by the validator before
distributing remaining funds to stake delegators. Previously 100% of block
rewards were distributed to the validator identity account.

- Inflation Rewards Commission: The commission rate that determines how much of 
stake inflation rewards are collected by the validator before distributing the
remaining rewards to stake delegators. Previously referred to as simply the
"commission" since there was no need to differentiate from other types of
commission.

## Detailed Design

Currently, all block fees, including both transaction base fees and priority
fees, are collected into a validator's node id account. This proposal details
changes to the Vote Program and Vote Account that will allow validators to
specify how different sources of income are collected. This proposal also
updates the bookkeeping for authorized voters in the vote account state.

### Vote Account 

A new version of vote state will be introduced with the enum discriminant value
of `3u32` which will be little endian encoded in the first 4 bytes of account
data. 

```rust
pub enum VoteStateVersions {
    V1(..),
    V2(..),
    V3(..),
    V4(..), // <- new variant
}
```

This new version of vote state will include new fields for setting the
commission and collector account for each of the sources of validator income:
inflation rewards and block rewards. It will also remove the `prior_voters`
field.

```rust
pub struct VoteStateV4 {
    pub node_pubkey: Pubkey,
    pub authorized_withdrawer: Pubkey,

    /// NEW: the collector accounts for validator income
    pub inflation_rewards_collector: Pubkey,
    pub block_rewards_collector: Pubkey,

    /// NEW: basis points (0-10,000) that represent how much of each income
    /// source should be given to this VoteAccount
    pub inflation_rewards_commission_bps: u16,
    pub block_rewards_commission_bps: u16,

    /// NEW: Rewards that will be distributed to stake delegators at the end of
    /// the epoch. These pending rewards are held in the DelegatorRewardsSysvar.
    pub pending_delegator_rewards: u64,

    /// REMOVED
    /// prior_voters: CircBuf<(Pubkey, Epoch, Epoch)>,

    pub votes: VecDeque<LandedVote>,
    pub root_slot: Option<Slot>,
    pub authorized_voters: AuthorizedVoters,
    pub epoch_credits: Vec<(Epoch, u64, u64)>,
    pub last_timestamp: BlockTimestamp,
}
```

Whenever a vote account is initialized OR modified by the vote program in a
transaction AND hasn't been updated to v4 yet, the account state MUST be saved
in the new format with the following default values for the new fields described
above:

```rust
VoteStateV4 {
    // ..

    inflation_rewards_collector: vote_account_pubkey,
    block_rewards_collector: vote_state_v3.node_pubkey,
    inflation_rewards_commission_bps: 100u16 * (vote_state_v3.commission as u16),
    block_rewards_commission_bps: 10_000u16,
    pending_delegator_rewards: 0u64,

    // ..
}
```

### Vote Program

```rust
pub enum VoteInstruction {
    // ..
    UpdateCommission {..} // 5u32
    // ..
    UpdateCommissionWithKind { // 16u32
        commission_bps: u16,
        kind: CollectorKind,
    },
    UpdateCollectorAccount { // 17u32
        pubkey: Pubkey,
        kind: CollectorKind,
    },
}

#[repr(u8)]
pub enum CollectorKind {
    InflationRewards = 0,
    BlockRewards,
}
```

#### `UpdateCommission`

The existing `UpdateCommission` instruction (with enum discriminant `5u32`) will
continue to be enabled but will continue to only update the inflation rewards
commission in integer percentage values.

#### `UpdateCommissionWithKind`

A new instruction for setting different kinds of commissions will be added to
the vote program with the enum discriminant of `16u32` little endian encoded in
the first 4 bytes.

#### `UpdateCollectorAccount`

A new instruction for setting collector accounts will be added to the vote
program with the enum discriminant value of `17u32` little endian encoded in the
first 4 bytes.

#### `Authorize`, `AuthorizeChecked`, `AuthorizeWithSeed`, `AuthorizeCheckedWithSeed`

Existing authorize instructions will be processed differently when setting new
authorized voters. Rather than purging authorized voter entries from the
`authorized_voters` field that correspond to epochs less than the current epoch,
only purge entries less than the previous epoch (current epoch - 1). This will
mean that the `authorized_voters` field can now hold up to 4 entries for the
epochs in the range `[current_epoch - 1, current_epoch + 2]`. Keeping the
authorized voter around from the previous epoch will allow the protocol to
accept votes from both the current and previous authorized voters to make voter
transitions smoother.

## Impact

This is a prerequisite for implementing block reward distribution in SIMD-0123.

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

Does the feature introduce any breaking changes? All incompatibilities and
consequences should be listed.