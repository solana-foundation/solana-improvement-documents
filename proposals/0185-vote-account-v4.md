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

Add a new version of vote account state to support alpenglow, block revenue
distribution, and commission improvements.

## Motivation

A new update for the vote program is proposed to improve authorized voter
bookkeeping as well as initialize state fields that will allow validators to set
commission rates and collector accounts for different revenue sources in the
future.

### Authorized voter bookkeeping

- Over 40% of vote state size is reserved for tracking a history of 32 prior
authorized voters for the vote account in the `prior_voters` field. Having such
a long history of prior voters is not useful, tracking the most recent previous
epoch's voter is sufficient for features like slashing and authorized voter
migration and can be stored in the `authorized_voters` field instead.

- The `authorized_voters` field doesn't store the voter for the previous epoch
so it's impossible to have a transition epoch where both the previous and newly
assigned voter can both sign votes.

### Revenue Collection Customization

- There is only one commission rate stored in vote account state but validators
want to be able to use different commission rates for different income streams
like block revenue.

- It's not possible to customize which accounts income is collected into.
Currently all block fee revenue is collected into the validator identity account
which cannot be a cold wallet since the identity needs to sign a lot of messages
for various network protocols used in Solana like turbine, gossip, and QUIC.

## Alternatives Considered

### Reuse vote commission

Vote accounts already allow validators to set a commission rate for inflation
rewards and so it's not unreasonable to expect that this commission rate could
also be used to distribute block revenue. However, some validators have
expressed a desire to be able to tune revenue streams independently.

## New Terminology

- Block Revenue Commission: The commission rate that determines how much of
block base fee and priority fee revenue is collected by the validator before
distributing remaining funds to stake delegators. Previously 100% of block
revenue was distributed to the validator identity account.

- Inflation Rewards Commission: The commission rate that determines how much of 
stake inflation rewards are collected by the validator before distributing the
remaining rewards to stake delegators. Previously referred to as simply the
"commission" since there was no need to differentiate from other types of
commission.

- Block Revenue Commission Collector: The account used to collect commissioned
block revenue for validators. Previously collected by default into the validator
identity account.

- Inflation Rewards Commission Collector: The account used to collect
commissioned inflation rewards for validators. Previously collected by default
into the vote account.

## Detailed Design

Currently, all block revenue, including both transaction base fees and priority
fees, is collected into a validator's node id account. This proposal details
changes to the vote account state that will allow validators to specify how
different sources of income are collected in future SIMD's. This proposal also
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
commission and collector account for the following sources of validator income:
inflation rewards and block revenue. It will also remove the `prior_voters`
field.

```rust
pub struct VoteStateV4 {
    pub node_pubkey: Pubkey,
    pub authorized_withdrawer: Pubkey,

    /// REMOVED
    /// commission: u8,

    /// NEW: the collector accounts for validator income
    pub inflation_rewards_collector: Pubkey,
    pub block_revenue_collector: Pubkey,

    /// NEW: basis points (0-10,000) that represent how much of each income
    /// source should be given to this VoteAccount
    pub inflation_rewards_commission_bps: u16,
    pub block_revenue_commission_bps: u16,

    /// NEW: reward amount pending distribution to stake delegators
    pub pending_delegator_rewards: u64,

    /// NEW: compressed bls pubkey for alpenglow
    pub bls_pubkey_compressed: Option<[u8; 48]>

    pub votes: VecDeque<LandedVote>,
    pub root_slot: Option<Slot>,

    /// UPDATED: serialization structure of the AuthorizedVoters map is
    /// unchanged but will now contain entries for the previous epoch.
    pub authorized_voters: AuthorizedVoters,

    /// REMOVED
    /// prior_voters: CircBuf<(Pubkey, Epoch, Epoch)>,

    pub epoch_credits: Vec<(Epoch, u64, u64)>,
    pub last_timestamp: BlockTimestamp,
}
```

### Vote Program

All vote instructions MUST be updated to support deserializing v4 vote accounts.

Whenever a vote account is initialized OR modified by the vote program in a
transaction AND hasn't been updated to v4 yet, the account state MUST be saved
in the new format with the following default values for the new fields described
above:

```rust
VoteStateV4 {
    // ..

    inflation_rewards_collector: vote_pubkey,
    block_revenue_collector: old_vote_state.node_pubkey,
    inflation_rewards_commission_bps: 100u16 * (old_vote_state.commission as u16),
    block_revenue_commission_bps: 10_000u16,
    pending_delegator_rewards: 0u64,
    bls_pubkey_compressed: None,

    // ..
}
```

If a modified vote account's size is smaller than `3762` bytes (only possible
for vote state versions v2 and earlier), first resize the account to `3762`
bytes before updating the account data. Then check whether the resulting account
is rent exempt or not and return an `AccountNotRentExempt` instruction error if
not rent exempt after the resize. This differs from the prior vote program
implementation which falls back to store vote state as v2 if the account would
not be rent exempt after its data length was increased.

#### `InitializeAccount`

The required size for vote accounts previously set to `3762` bytes MUST remain
unchanged despite freeing up space with the removal of the prior voters field.
Keeping the same size requirement simplifies this proposal and leaves extra
space for future fields.

Note that a v4 vote account is ALWAYS considered initialized, because unlike
other vote state versions, it's never stored with uninitialized state.

#### `UpdateCommission`

The existing `UpdateCommission` instruction will will continue to only update
the inflation rewards commission in integer percentage values.

When updating vote state v4 accounts, the new `inflation_rewards_commission_bps`
field should be used instead of the old generic `commission` field.
Additionally, the new commission value MUST be multiplied by `100` before being
checked for commission or increases and before being stored in account data.

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

Additionally, since the `prior_voters` field is removed from vote state v4,
there's no need to read or modify this field when processing authorize
instructions.

#### `Withdraw`

The existing withdraw instruction MUST be modified to completely zero vote
account data for fully withdrawn vote accounts. The old behavior partially
zeroed the account data following the vote state version discriminant and is
less intuitive.

### Stake Program

The builtin stake program reads vote account state when creating, delegating,
and deactivating stake accounts. The program MUST be updated to support v4 vote
accounts.

### Runtime

Commission rates will now be stored in basis points but the pre-existing
`UpdateCommission` instruction only supports integer percentage commission
values (higher precision values cannot be set until [SIMD-0291] is adopted).
So from the runtime's perspective, commission rates will remain limited to
multiples of 100 basis points, equivalent to integer percentages. Therefore,
commission calculations should continue to use integer percentage values for
now.

The runtime stakes cache and epoch stakes stored in snapshots MUST also be
updated to support initialized v4 vote accounts.

[SIMD-0291]: https://github.com/solana-foundation/solana-improvement-documents/pull/291

### Other

The tower serialization format MUST remain unchanged and continue serializing tower
vote state as vote state v2.

## Impact

This is a prerequisite for implementing other SIMD's like block revenue
distribution in [SIMD-0123] which give validators more flexibility in how
inflation rewards and block revenue is collected and distributed.

[SIMD-0123]: https://github.com/solana-foundation/solana-improvement-documents/pull/123

## Security Considerations

NA

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

Existing programs that read vote state will need to be updated to support the
latest account state version.
