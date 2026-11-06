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
5. Renames `epoch_credits` to `accumulated_lamports` and keeps only the
   last 5 entries.
6. Dynamically-sized fields are now statically-sized for zero-copy access
7. Reduces the account size from 3762 bytes to 951 bytes.
8. Adds a `rank` field, which is just a placeholder in this SIMD.

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
[SIMD-0387]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0387-bls-pubkey-management-in-vote-account.md
[SIMD-0674]: https://github.com/solana-foundation/solana-improvement-documents/pull/674
[SIMD-0464]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0464-vote-account-initialize-v2.md

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
| 132 | 12 | location | `x: i32, y: i32, z: i32` |
| 144 | 8 | pending_delegator_rewards | `u64` |
| 152 | 4 | inflation_rewards_commission_bps_len | `u32`, 1 to 4 |
| 156 | 4 | block_revenue_commission_bps_len | `u32`, 1 to 4 |
| 160 | 32 | inflation_rewards_commission_bps_epochs | `[u64; 4]` |
| 192 | 8 | inflation_rewards_commission_bps | `[u16; 4]` |
| 200 | 32 | block_revenue_commission_bps_epochs | `[u64; 4]` |
| 232 | 8 | block_revenue_commission_bps | `[u16; 4]` |
| 240 | 48 | bls_pubkey_compressed | `[u8; 48]` |
| 288 | 8 | latest_vote | `u64`, `u64::MAX` = none |
| 296 | 8 | root_slot | `u64`, `u64::MAX` = none |
| 304 | 4 | authorized_voters_len | `u32`, at most 4 |
| 308 | 4 | accumulated_lamports_len | `u32`, at most 5 |
| 312 | 160 | authorized_voters | `[(epoch: u64, pubkey: Pubkey); 4]` |
| 472 | 120 | accumulated_lamports | `[(epoch, lamports, prev_lamports); 5]` |
| 592 | 16 | last_timestamp | `(slot: u64, timestamp: i64)` |
| 608 | 10 | rank | `(epoch: u64, rank: u16)`, `(u64::MAX, u16::MAX)` = none |
| 618 | 333 | zeroed | `[u8; 333]`, all zero |

All integers are little-endian. Note that the account size is 951
bytes, which is smaller than the 3762 bytes of Vote Account v4. There
is no padding. Each field starts at a multiple of its own size:
all `u64` and `i64`'s at a multiple of 8 bytes and all `u32` and
`i32`'s at a multiple of 4 bytes.

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
- `rank` - the validator's Alpenglow rank and the epoch during which
  it held this rank. This is an optional field - if it's not set,
  the epoch is `u64::MAX` and the rank is `u16::MAX`. This field is not
  set in this SIMD, but will be updated in a future SIMD.
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
  instead of credits. It is now a fixed-size array of up to 5 entries,
  oldest first. `accumulated_lamports_len` holds the number of these
  entries. Under Alpenglow every delegation is paid its rewards every
  epoch, so rewards only need the last 2 entries. However, the Stake
  program's DeactivateDelinquent instruction uses 5 entries to determine
  if the vote account has voted recently. We will reduce the length of
  this field to 2 in a future SIMD.

All other fields are unchanged.

### Validation

A Vote Account v5 MUST fail to deserialize unless all of following are
true:

- The account data length is exactly 951 bytes.
- `location` is valid as defined in [SIMD-0674].
- The inflation rewards commission rate fields hold 1 to 4 commission
  rate changes.
- The block revenue commission rate fields hold 1 to 4 commission
  rate changes.
- `authorized_voters` has at most 4 entries with strictly increasing
  epochs.
- `accumulated_lamports` has at most 5 entries.
- `rank` is `(u64::MAX, u16::MAX)`.
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

If the last entry in the array is already for epoch `E + 2`, the `bps`
value is overwritten. Otherwise the new entry is appended to the end
of the list. If there are already 4 changes, the oldest one is removed
first.

Old entries are not removed when their epoch passes. An entry is only
removed when a new change pushes it out.

The rate for an epoch `X` is the `bps` of the entry with the highest
epoch that is not after `X`.

This SIMD does not change the semantics of how much delay each commission
update incurs. It only records extra data into the vote account so that
on-chain programs can use this.

#### Example 1

If there are already 4 entries in a commission rate array:

| Epoch | 0 | 1 | 2 | 3 |
|-------|--:|--:|--:|--:|
| Rate | 10 | 11 | 12 | 13 |

and a `[E=2, rate=14]` commission rate update occurs, then the oldest
entry is removed and the new one is added at the end:

| Epoch | 1 | 2 | 3 | 4 |
|-------|--:|--:|--:|--:|
| Rate | 11 | 12 | 13 | 14 |

If no more changes are made, the array stays the same in future epochs.

### Conversion from Vote Account v4

Conversion from Vote Account v4 to Vote Account v5 only happens with
the `UpdateLocation` Vote program instruction introduced
in [SIMD-0674]. This is because `location` is a mandatory Vote Account v5
field. Therefore SIMD does not have a feature gate of its own, and does
not change any on-chain logic before the [SIMD-0674] feature gate
is activated. This SIMD simply to describes the format, semantics of
the fields and conversion from Vote Account v4 to Vote Account v5, to
avoid bloating [SIMD-0674].

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
| accumulated_lamports | the last 5 `epoch_credits` entries, in order |
| last_timestamp | copied |
| rank | `(u64::MAX, u16::MAX)` |
| zeroed | all zero |

A Vote Account v4 only stores its latest rate, so the conversion
records it as `(E + 2, rate)`, even if the rate was set long ago. The
rates for epochs `E` and `E + 1` are not stored in the vote account.
Rewards for those epochs use the rates the runtime saved at the earlier
epoch boundaries, as they do today.

`UpdateLocation` MUST only convert a 3762-byte Vote Account v4, and
MUST then shrink it to 951 bytes. The account keeps its lamports.

The `UpdateLocation` instruction MUST fail with `InvalidAccountData` if
the Vote Account v4 has no BLS key. Note that this means that
conversion from vote account versions older than v4 will fail, because
they do not have a BLS key set. So older vote accounts must first be
converted to Vote Account v4, and set a valid BLS key, before they can
be converted to Vote Account v5.

An account can be converted before it has voted in Alpenglow. Its
`latest_vote`, `root_slot` and `last_timestamp` then still hold its old
Tower values, which are replaced once it starts voting in Alpenglow. New
lamports are added on top of its last Tower entry in
`accumulated_lamports`. See [Runtime Changes](#runtime-changes) and
[Accumulated Lamports](#accumulated-lamports) for details.

#### Example 2

If a Vote Account v4 with a stored commission rate of 500 in the
account state is converted to Vote Account v5 during epoch 100, the
commission array will be:

| Epoch | 102 |
|-------|----:|
| Rate | 500 |

This does not necessarily mean that the rate changed in epoch 100,
just that the previous historical values from before the conversion
are not stored on-chain.

#### Example 3

If a Vote Account v4 has had its commission rate changed from 500 to
700 during epoch 99, and is then converted to a Vote Account v5 during
epoch 100, the commission array will be:

| Epoch | 102 |
|-------|----:|
| Rate | 700 |

The change made in epoch 99 takes effect in epoch 101, so the rates
actually used by the runtime are:

| Epoch | 100 | 101 | 102 |
|-------|----:|----:|----:|
| Effective Rate | 500 | 700 | 700 |

The array does not show that 500 was used in epoch 100, or that 700 is
already used in epoch 101.

### Creating a Vote Account v5

There is no instruction that creates a Vote Account v5 directly. To
create one:

1. Create a Vote Account v4 with `InitializeAccount`.
2. Set the BLS key as described in [SIMD-0387].
3. Call `UpdateLocation` as described in [SIMD-0674], to convert it to
   a Vote Account v5.

A future SIMD could add an instruction that creates a Vote Account v5
in one step, similar to [SIMD-0464].

### Vote Program Changes

All Vote program instructions MUST support Vote Account v5. The
semantics of some instructions MUST change:

- `UpdateLocation` is the new instruction described in [SIMD-0674], and
  is the only instruction which can convert an account to Vote Account
  v5, as described in [Conversion from Vote Account v4](#conversion-from-vote-account-v4).
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
    and `last_timestamp` for the reward slot. For a Vote Account v4,
    this replaces the `votes` field with a single vote for the reward
    slot. A Vote Account v5 has no `votes` field, so the runtime sets
    `latest_vote` instead, to the highest of its current value, the
    reward slot and `root_slot`. `last_timestamp` is only changed if the
    reward slot's timestamp is later than the one stored, and its slot
    and timestamp are changed together. The runtime also adds the
    validator's share of the reward to `accumulated_lamports` as
    described in [Accumulated Lamports](#accumulated-lamports). The
    leader's share is added to the leader's `accumulated_lamports`.
  - Finalization: when processing an Alpenglow finalization certificate,
    for each validator in the certificate the runtime first sets
    `root_slot` to the highest of its current value and the finalized
    slot. It then updates `latest_vote` and `last_timestamp` for the
    finalized slot, in the same way as for Vote Rewards.
  - Block Revenue and Delegator Rewards: at the end of each block, the
    delegators' share of the fees is added to the leader's
    `pending_delegator_rewards`. At the start of each epoch,
    `pending_delegator_rewards` is paid out to stake accounts and reset
    to 0.

  In these updates, a `latest_vote` or `root_slot` of `u64::MAX` (none)
  counts as 0. Therefore, `latest_vote`, `root_slot` and `last_timestamp`
  monotonically increase.

  If a validator is in both the reward certificate and the finalization
  certificate of a block, the reward certificate is processed first.

  Commission rate selection is unchanged: the rate is still read from the
  vote account state in the validators internal cache, as in [SIMD-0249] and [SIMD-0123].
  For a Vote Account v5, the rate is the `bps` of the last entry in each
  commission array, in the cached vote account. See [Commission Rate Changes](#commission-rate-changes).

### Accumulated Lamports

`accumulated_lamports` is updated the same way as `epoch_credits` in
Vote Account v4. Each entry is `(epoch, lamports, prev_lamports)`, where
`lamports` is a running total and `prev_lamports` is the total before
this entry.

To add `n` lamports in epoch `E`, where `total` is the total of the
last entry (0 if there are none), the runtime:

- adds `n` to the last entry, if it is for epoch `E`.
- otherwise, if the last entry earned nothing, changes its epoch to `E`
  and adds `n` to it.
- otherwise, adds a new entry `(E, total + n, total)`, dropping the
  oldest entry if there are already 5.

In the Alpenglow migration epoch, the first time lamports are added, a
marker entry `(u64::MAX, u64::MAX, u64::MAX)` is added first, unless
there is one already. It counts towards the 5 entries. Entries before
the marker are Tower credits and entries after it are Alpenglow
lamports. When the last entry is the marker, a new entry is always
added, and `total` is the total of the entry before the marker.

The conversion copies the last 5 `epoch_credits` entries as they are,
including the marker if there is one. If the account has not earned any
Alpenglow lamports yet, it has no marker, and the rules above simply
carry on from its last Tower entry.

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

### Vote Account Size Checks

Several places in the runtime check for a 3762 byte account size to
determine if an account is a valid Vote Account v4. These call sites
MUST be updated to also support Vote Account v5's new smaller size.
In particular:

- Validators keep a list of vote accounts, used for epoch stakes, the
  leader schedule, the VAT and rewards. An account only goes on this list
  if its size and discriminant match a known vote account version. A
  951-byte account with discriminant `4` MUST pass this check, whether it
  is being written by a block or loaded from genesis or a snapshot.
- Crediting and paying out `pending_delegator_rewards` MUST accept a
  951-byte Vote Account v5.
- The VAT minimum balance MUST be the rent-exempt minimum for the vote
  account's actual size, plus one epoch of VAT burn. For a Vote Account
  v4 this is the same as today.

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

- Keep the 3762-byte size of Vote Account v4. However, this is
  unnecessary and there are performance benefits to hashing smaller
  accounts. 951 bytes leaves a bit of room for extra fields in the
  future without having to worry about changing the rent-exempt value
  for vote accounts.
- Set each validator's Alpenglow rank in this SIMD. This is a bigger
  change to the runtime, so a future SIMD will set the `rank` field.

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
  anyone to upgrade. Upgrading is optional until [SIMD-0674]'s
  `enforce_validator_location_registration` feature gate is activated,
  which excludes vote accounts without a location set - and therefore
  have a vote account version older than v5 - from the VAT.

## Conformance

A series of test fixtures MUST be provided, using the
[conformance tooling](https://github.com/anza-xyz/agave/wiki/Conformance-Tools).
These MUST include the following scenarios at a minimum: conversion from
Vote Account v4 (including no BLS key and the resize to 951 bytes),
commission updates in one and in consecutive epochs and all Vote Account
v5 states that fail validation.
