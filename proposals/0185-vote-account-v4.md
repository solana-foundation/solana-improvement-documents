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

A new update for the vote program is proposed to free up state space used for
unnecessary bookkeeping.

## Motivation

Over 40% of vote state size is reserved for tracking a history of 32 prior
authorized voters in the `prior_voters` field. Having such a long history of
prior voters is not useful onchain and for offchain usecases this information
can be fetched via other means.

By removing the `prior_voters` field, the space in vote accounts can be used for
future field additions to vote state without needing to reallocate existing vote
accounts.

## Alternatives Considered

NA

## New Terminology

NA

## Detailed Design

### Vote Account 

A new version of vote state will be introduced with the enum discriminant value
of `3u32` which will be little endian encoded in the first 4 bytes of account
data. 

```rust
pub enum VoteStateVersions {
    V1(..),
    V2(..),
    V3(..),
    V4(VoteStateV4), // <- new variant
}
```

This new version of vote state will remove the `prior_voters` field.

```rust
pub struct VoteStateV4 {
    pub node_pubkey: Pubkey,
    pub authorized_withdrawer: Pubkey,
    pub commission: u8,
    pub votes: VecDeque<LandedVote>,
    pub root_slot: Option<Slot>,
    pub authorized_voters: AuthorizedVoters,

    /// REMOVED
    /// prior_voters: CircBuf<(Pubkey, Epoch, Epoch)>,

    pub epoch_credits: Vec<(Epoch, u64, u64)>,
    pub last_timestamp: BlockTimestamp,
}
```

### Vote Program

Whenever a vote account is modified by the vote program in a transaction AND
hasn't been updated to v4 yet, the account state MUST be saved in the new
format. New vote accounts should still be allocated with the same size (3762
bytes) as the prior vote state version to keep extra space for future field
additions.

#### `Authorize`, `AuthorizeChecked`, `AuthorizeWithSeed`, `AuthorizeCheckedWithSeed`

Since `prior_voters` will be removed in the new vote state version, existing
authorize instructions will no longer need to update `prior_voters` when setting
new authorized voters.

## Impact

This is a prerequisite for implementing custom block fee collection in SIMD-0232.

## Security Considerations

NA

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

A new feature gate will be added to switch over vote accounts to the new state version.
