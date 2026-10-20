---
simd: '0701'
title: Vote Account v5
authors:
  - Tom Pointon (Firedancer)
  - Umar Bhatty
category: Standard
type: Core
status: Idea
created: 2026-10-20
feature: (does not have its own feature gate)
supersedes: SIMD-0677
extends: SIMD-0185
---

## Summary

Introduces Vote Account v5, the successor to Vote Account v4. This is a new
vote account layout designed to facilitate zero-copy access, support
the geo-aware leader schedule and include more information about
commission rate changes. We take the opportunity to clean up some tech
debt from Vote Account v4.

Differences from Vote Account v4:

1. New mandatory `location` field, for [SIMD-0674].
2. `bls_pubkey_compressed` is now mandatory.
3. Replaces the `votes` array with a single optional `latest_vote`.
4. Replaces each commission rate with a commission schedule.
5. Renames `epoch_credits` to `accumulated_lamports`.

## Motivation

There are several reasons to make these changes.

- [SIMD-0674] needs a place to store each validator's location on-chain.
- Under [SIMD-0249], a commission change made in epoch E takes effect
  from epoch E+2. On-chain programs therefore have no visibility into
  the effective commission rate of the vote account, or into when
  queued-up commission rate changes take effect. Storing up to 4
  commission rate changes, with the epoch in which each takes effect,
  gives on-chain programs visibility into the previous epoch's rate,
  the current rate and up to two pending rate changes.
- Under Alpenglow, historical `votes` are not used, `epoch_credits`
  have a different meaning and a `bls_pubkey_compressed` is needed for
  all vote accounts.

## Dependencies

- [SIMD-0185] Vote Account v4 is the starting point for Vote Account v5.
- [SIMD-0249] Delay Commission Updates and [SIMD-0123] Block Revenue
  Distribution define when a commission change takes effect.
- [SIMD-0232] Custom Commission Collector defines the vote account
  state that the inflation rewards are calculated from.
- [SIMD-0384] Alpenglow changes the meaning of `votes` and
  `epoch_credits` and requires `bls_pubkey_compressed` to be set.
- [SIMD-0674] Validator Location Registration defines the semantics of
  the location field and introduces the instruction which converts a
  Vote Account v4 into a Vote Account v5.

[SIMD-0123]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0123-block-revenue-distribution.md
[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0185-vote-account-v4.md
[SIMD-0232]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0232-custom-commission-collector.md
[SIMD-0249]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0249-delay-commission-updates.md
[SIMD-0291]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0291-commission-rate-in-basis-points.md
[SIMD-0384]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0384-alpenglow-migration.md
[SIMD-0674]: https://github.com/solana-foundation/solana-improvement-documents/pull/674

## New Terminology

None.

## Detailed Design

### Layout

The new Vote Account v5 state layout:

| Offset | Size | Field | Type |
|-------:|-----:|-------|------|
| 0 | 4 | discriminant | `u32` = `4` |
| 4 | 32 | node_pubkey | `Pubkey` |
| 36 | 32 | authorized_withdrawer | `Pubkey` |
| 68 | 32 | inflation_rewards_collector | `Pubkey` |
| 100 | 32 | block_revenue_collector | `Pubkey` |
| 132 | 8 | pending_delegator_rewards | `u64` |
| 140 | 12 | location | `x: i32, y: i32, z: i32` |
| 152 | 4 | inflation_rewards_commission_bps_len | `u32`, 1 to 4 |
| 156 | 32 | inflation_rewards_commission_bps_epochs | `[u64; 4]` |
| 188 | 8 | inflation_rewards_commission_bps | `[u16; 4]` |
| 196 | 4 | block_revenue_commission_bps_len | `u32`, 1 to 4 |
| 200 | 32 | block_revenue_commission_bps_epochs | `[u64; 4]` |
| 232 | 8 | block_revenue_commission_bps | `[u16; 4]` |
| 240 | 48 | bls_pubkey_compressed | `[u8; 48]` |
| 288 | 8 | latest_vote | `u64`, `u64::MAX` = none |
| 296 | 8 | root_slot | `u64`, `u64::MAX` = none |
| 304 | 4 | authorized_voters_len | `u32`, at most 4 |
| 308 | 160 | authorized_voters | `[(epoch: u64, pubkey: Pubkey); 4]` |
| 468 | 4 | accumulated_lamports_len | `u32`, at most 64 |
| 472 | 1536 | accumulated_lamports | `[(epoch, lamports, prev_lamports); 64]` |
| 2008 | 16 | last_timestamp | `(slot: u64, timestamp: i64)` |
| 2024 | 1738 | zeroed | `[u8; 1738]`, all zero |

All integers are little-endian. The size of the vote account is
unchanged from Vote Account v4. There is no padding. Each field starts at
a multiple of 4 bytes.

### New Fields

- `location` - the validator's location. See [SIMD-0674].
- `inflation_rewards_commission_bps_len`,
  `inflation_rewards_commission_bps_epochs` and
  `inflation_rewards_commission_bps` replace the Vote Account v4
  `inflation_rewards_commission_bps` field. These hold between 1 and 4
  inflation rewards commission rate changes and the epoch in which each
  takes effect.
  Change `i` means "commission rate
  `inflation_rewards_commission_bps[i]` applies from epoch
  `inflation_rewards_commission_bps_epochs[i]`." See
  [Commission Rate Changes](#commission-rate-changes).
- Similarly, `block_revenue_commission_bps_len`,
  `block_revenue_commission_bps_epochs` and
  `block_revenue_commission_bps` contain the changes to the block
  revenue commission, replacing `block_revenue_commission_bps`.
- `latest_vote` replaces `votes`. It contains the slot number of the
  vote account's latest vote, or `u64::MAX` if the vote account hasn't
  voted.
- `zeroed` isn't technically a field, but zeroed padding at the end of
  the vote state.

### Changed Fields

The other fields have the same semantics and meaning as in Vote Account
v4. There are some encoding changes to support zero-copy access:

- `bls_pubkey_compressed` is now mandatory.
- `root_slot` is now `u64::MAX` when there is no root, instead of
  being serialized as a bincode `Option`.
- `authorized_voters` is a fixed-size array of up to 4 entries in
  increasing epoch order. `authorized_voters_len` holds the number of
  these entries.
- `epoch_credits` is renamed to `accumulated_lamports` to reflect the
  changed semantics post-Alpenglow: this array now holds lamports
  instead of credits. It is now a fixed-size array of up to 64 entries,
  oldest first. `accumulated_lamports_len` holds the number of these
  entries.

All other fields are unchanged.

### Validation

A Vote Account v5 MUST fail to deserialize unless all of following are
true:

- The account data length is exactly 3762 bytes.
- `location` is valid as defined in [SIMD-0674].
- The inflation rewards commission rate fields hold 1 to 4 commission
  rate changes, with strictly increasing epochs.
- The block revenue commission rate fields hold 1 to 4 commission
  rate changes, with strictly increasing epochs.
- `authorized_voters` has at most 4 entries with strictly increasing
  epochs.
- `accumulated_lamports` has at most 64 entries.
- For every fixed size list, each entry at or after its length is zero.
- `zeroed` is all zero.

### Commission Rate Changes

The only way commission rate changes are made is through the Vote
program `UpdateCommission` and `UpdateCommissionBps` instructions. See
[SIMD-0291].

When these instructions succeed for epoch `E`, the Vote program MUST
record the change `(E + 2, bps)` in the appropiate commission array.
This is because, as per [SIMD-0249], the new rate will take effect at
epoch `E + 2`.

If the latest entry in the array is already for epoch `E + 2`, the `bps`
value is overwritten. Otherwise the new entry is appended to the front
of the list. If there are already 4 changes, the oldest one is removed
first.

This SIMD does not change the semantics of how much delay each commission
update incurs. It only records extra data into the vote account so that
on-chain programs can use this.

### Conversion from Vote Account v4

Conversion from Vote Account v4 to Vote Account v5 only happens with
the `UpdateLocation` Vote program instruction introduced
in [SIMD-0674]. This is because `location` is a mandatory Vote Account v5
field. Therefore SIMD does not have a feature gate of its own, but
instead takes effect when the [SIMD-0674] feature gate is activated.

The `UpdateLocation` instruction converts the Vote Account v4 state to
the new Vote Account v5 state using the conversion function below for
each field, where `E` is the current epoch:

| Field | Value |
|-------|-------|
| node_pubkey … block_revenue_collector | copied |
| pending_delegator_rewards | copied |
| location | the location from the `UpdateLocation` instruction |
| inflation rewards commission | one entry: `(E + 2, rate)` |
| block revenue commission | one entry: `(E + 2, rate)` |
| bls_pubkey_compressed | copied |
| latest_vote | slot of the last `votes` entry, or `u64::MAX` if none |
| root_slot | copied, or `u64::MAX` if `None` |
| authorized_voters | copied, in increasing epoch order |
| accumulated_lamports | the `epoch_credits` entries, in order |
| last_timestamp | copied |
| zeroed | all zero |

The `UpdateLocation` instruction MUST fail with `InvalidAccountData` if
the Vote Account v4 has no BLS key, more than 4 authorized voters or
more than 64 `epoch_credits` entries. Note that this means that
conversion from vote account versions older than v4 will fail, because
they do not have a BLS key set. So older vote accounts must first be
converted to Vote Account v4, and set a valid BLS key, before they can
be converted to Vote Account v5.

### Vote Program Changes

All Vote program instructions MUST support Vote Account v5. The
semantics of some instructions MUST change:

- `UpdateLocation` is the new instruction described in [SIMD-0674], and
  is the only instruction which can convert an account to Vote Account
  v5, as described in [Conversion from Vote Account v4](#conversion-from-vote-account-v4).
- `Vote`, `VoteSwitch`, `UpdateVoteState`,
  `UpdateVoteStateSwitch`, `CompactUpdateVoteState`,
  `CompactUpdateVoteStateSwitch`, `TowerSync`, `TowerSyncSwitch` MUST
  fail with `InvalidInstructionData` if they are passed a Vote Account
  v5.
- `Authorize`, `AuthorizeChecked`, `AuthorizeWithSeed` and
  `AuthorizeCheckedWithSeed` MUST fail with `InvalidInstructionData`
  if, after purging, adding an authorized voter would exceed
  the limit of 4 authorized voters.
- `UpdateCommission` and `UpdateCommissionBps` MUST behave as described
  above in [Commission Rate Changes](#commission-rate-changes).

### Runtime Changes

  Whenever the runtime updates a vote account's state, it MUST keep the
  account a Vote Account v5 and update its fields. This happens in the
  following places:

  - Vote Rewards: when processing an Alpenglow reward certificate, for
    each validator in the certificate the runtime updates `latest_vote`
    and `last_timestamp` for the reward slot, and adds the validator's
    share of the reward to `accumulated_lamports`. The leader's share is
    added to the leader's `accumulated_lamports`.
  - Finalization: when processing an Alpenglow finalization certificate,
    for each validator in the certificate the runtime updates `root_slot`,
    `latest_vote` and `last_timestamp` for the finalized slot.
  - Block Revenue and Delegator Rewards: at the end of each block, the
    delegators' share of the fees is added to the leader's
    `pending_delegator_rewards`. At the start of each epoch,
    `pending_delegator_rewards` is paid out to stake accounts and reset
    to 0.

  Commission rate selection is unchanged: the rate is still read from the
  vote account state in the validators internal cache, as in [SIMD-0249] and [SIMD-0123].
  This may change in a future SIMD.

### Stake Program Changes

The on-chain core BPF stake program needs to be updated to read
Vote Account v5 accounts. Specifically:

- `DelegateStake` reads the credits of the latest `epoch_credits` entry
  to set the stake's `credits_observed`.
- `DeactivateDelinquent` reads the recent `epoch_credits` entries of the
  delinquent and reference vote accounts.

Both instructions MUST accept a Vote Account v5 and read
`accumulated_lamports` instead of `epoch_credits`.

When [SIMD-0674] is activated, the runtime MUST upgrade the Stake
program to a version that does this.

### Validator Components Affected

Every component that reads or writes vote accounts MUST support Vote
Account v5:

| Validator Component             | Impact                                  |
|---------------------------------|-----------------------------------------|
| Transaction Execution (Runtime) | Vote program; vote account reads/writes |
| Virtual Machine                 | None                                    |
| Block Packing                   | None                                    |
| Consensus                       | Alpenglow vote account updates          |
| Gossip                          | None                                    |
| Turbine                         | None                                    |
| Snapshots                       | Manifest stakes and epoch stakes        |
| On-Chain Core BPF Programs      | Stake program reads                     |
| Other (RPC)                     | `getVoteAccounts`, parsed vote accounts |

## Alternatives Considered

- Shrink the Vote Account v5 data length to 2024 bytes. As in Vote
  Account v4, we opted to keep the length constant to avoid vote accounts
  dropping below the rent-exempt value if more fields are added in the
  future.
- Store each validator's Alpenglow rank. This is a bigger change to the
  runtime, so this will come in a future version.

## Impact

- All third-party programs and tools that parse vote accounts MUST
  support Vote Account v5.

## Security Considerations

- Commission rate selection is unchanged, so this SIMD does not change
  the commission any validator is paid.
- Every Vote Account v5 field is checked on deserialization, so a
  malformed account cannot be read in place by mistake.

## Backwards Compatibility

- Vote Account v4 accounts remain valid, this SIMD does not force
  anyone to upgrade.

## Conformance

A series of test fixtures MUST be provided, using the
[conformance tooling](https://github.com/anza-xyz/agave/wiki/Conformance-Tools).
These MUST include the following scenarios at a minimum: conversion from
Vote Account v4 (including no BLS key and too many authorized voters or
`epoch_credits` entries), commission updates in one and in consecutive
epochs and all Vote Account v5 states that fail validation.
