---
simd: '0677'
title: Vote Account v5
authors:
  - Umar Bhatty
category: Standard
type: Core
status: Idea
created: 2026-09-30
feature: (fill in with feature key and github tracking issues once accepted)
extends: SIMD-0185
---

## Summary

Introduce vote state version 5. It makes three changes to the v4 layout:

1. The two commission rate fields are replaced by small fixed-size
   commission schedules. The Vote program records each commission change
   together with the epoch it takes effect.
2. A validator location field is added for [SIMD-0674].
3. The `votes` field, which is unused under Alpenglow, is removed.

## Motivation

Three pending changes each need a new vote state version. Defining them
together avoids competing proposals for the same version discriminant and
a second migration of every vote account.

**Commission.** [SIMD-0249] and [SIMD-0123] delay commission updates:
rewards earned in epoch `E` use the rate from the vote account state at
the beginning of epoch `E - 1`. The rate that applies to an epoch is
therefore no longer in the vote account. The account holds the rate that
will apply two epoch boundaries from now, and the rate in force is read
from state of an earlier epoch boundary that validator clients keep
outside the accounts database ([discussion 583]). An on-chain program
cannot read the rate in force at all, and some stake pools work around
this by sampling commission every epoch. Storing each change in the
vote account with its effective epoch makes the rate in force and the
pending rates readable from the account.

**Location.** [SIMD-0674] registers a validator's location in its vote
account and, as drafted, defines the next vote state version itself.

**Unused fields.** [SIMD-0357] states that the `votes` list in vote state
will be empty under Alpenglow.

## Dependencies

- **[SIMD-0185]: Vote Account v4**: the layout and conversion pattern this
  proposal starts from.
- **[SIMD-0249]: Delay Commission Updates** and **[SIMD-0123]: Block
  Revenue Sharing**: define when a commission update takes effect. This
  proposal keeps that timing.
- **[SIMD-0232]: Custom Commission Collector**: defines the vote account
  state that inflation rewards are calculated from.
- **[SIMD-0291]: Commission Rate in Basis Points** and **[SIMD-0464]: Vote
  Account Initialize V2**: instructions that write the commission.
- **[SIMD-0384]: Alpenglow Migration**: removing `votes` requires that
  tower votes are no longer recorded in vote accounts.

[SIMD-0674] is not a dependency. It currently defines its own v5 layout
and would need to be amended to take the layout from this proposal. The
location field stays directly after `pending_delegator_rewards`. Its byte
offset becomes 202, where SIMD-0674 has 144.

[SIMD-0118]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0118-partitioned-epoch-reward-distribution.md
[SIMD-0123]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0123-block-revenue-distribution.md
[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0185-vote-account-v4.md
[SIMD-0232]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0232-custom-commission-collector.md
[SIMD-0249]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0249-delay-commission-updates.md
[SIMD-0291]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0291-commission-rate-in-basis-points.md
[SIMD-0357]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0357-alpenglow_validator_admission_ticket.md
[SIMD-0384]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0384-alpenglow-migration.md
[SIMD-0464]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0464-vote-account-initialize-v2.md
[SIMD-0674]: https://github.com/solana-foundation/solana-improvement-documents/pull/674
[discussion 583]: https://github.com/solana-foundation/solana-improvement-documents/discussions/583

## New Terminology

- **Commission schedule**: a fixed-size list of at most three
  `(epoch, bps)` entries in the vote account, one schedule per commission
  kind.
- **Effective epoch**: the `epoch` of a schedule entry. The entry's rate
  applies to rewards from that epoch on, at the latest.
- **Commission of a vote state for epoch `E`**: the rate a given vote
  state yields for rewards earned in epoch `E`, defined under
  [Commission Schedule](#commission-schedule).

## Detailed Design

### Vote Account

A new version of vote state is introduced with the enum discriminant value
`4u32`, little endian encoded in the first 4 bytes of account data.

```rust
pub struct CommissionEntry {
    /// Epoch from which `bps` applies, at the latest.
    pub epoch: Epoch,
    pub bps: u16,
}

pub struct CommissionSchedule {
    /// Number of entries in use, from 1 to 3.
    pub len: u8,
    /// Entries `[0, len)` are in strictly increasing `epoch` order.
    /// Entries `[len, 3)` are zero.
    pub entries: [CommissionEntry; 3],
}

pub struct VoteStateV5 {
    pub node_pubkey: Pubkey,
    pub authorized_withdrawer: Pubkey,
    pub inflation_rewards_collector: Pubkey,
    pub block_revenue_collector: Pubkey,

    /// REPLACED: was `inflation_rewards_commission_bps: u16`
    pub inflation_rewards_commission: CommissionSchedule,
    /// REPLACED: was `block_revenue_commission_bps: u16`
    pub block_revenue_commission: CommissionSchedule,

    pub pending_delegator_rewards: u64,

    /// NEW: ECEF coordinates in meters as defined by SIMD-0674.
    /// (0, 0, 0) means that no location is registered.
    pub location: (i32, i32, i32),

    pub bls_pubkey_compressed: Option<[u8; 48]>,

    /// REMOVED
    /// votes: VecDeque<LandedVote>,

    pub root_slot: Option<Slot>,
    pub authorized_voters: AuthorizedVoters,
    pub epoch_credits: Vec<(Epoch, u64, u64)>,
    pub last_timestamp: BlockTimestamp,
}
```

All fields are serialized like their v4 counterparts. A `CommissionEntry`
is 10 bytes (`epoch` as `u64`, `bps` as `u16`, little endian). A
`CommissionSchedule` is 31 bytes: `len` followed by the three entries, with
no length prefix. Fields up to and including `location` sit at fixed byte
offsets:

| Offset | Size | Field                          |
|--------|------|--------------------------------|
| 0      | 4    | version discriminant (`4`)     |
| 4      | 32   | `node_pubkey`                  |
| 36     | 32   | `authorized_withdrawer`        |
| 68     | 32   | `inflation_rewards_collector`  |
| 100    | 32   | `block_revenue_collector`      |
| 132    | 31   | `inflation_rewards_commission` |
| 163    | 31   | `block_revenue_commission`     |
| 194    | 8    | `pending_delegator_rewards`    |
| 202    | 12   | `location` (`x`, `y`, `z`)     |
| 214    | 1/49 | `bls_pubkey_compressed`        |

The maximum serialized size of `VoteStateV5` is 2000 bytes. The required
vote account size of `3762` bytes MUST remain unchanged.

### Commission Schedule

`effective(E)` returns the entry in `[0, len)` with the greatest `epoch`
that is less than or equal to `E`, or nothing if there is no such entry.

`schedule(t, bps)` records that `bps` applies from epoch `t`:

```rust
fn schedule(&mut self, t: Epoch, bps: u16) {
    let last = self.len - 1;
    if self.entries[last].bps == bps {
        return; // no change
    }
    if self.entries[last].epoch == t {
        self.entries[last].bps = bps; // same target epoch: overwrite
        return;
    }
    if self.len == 3 {
        self.remove(0); // the other entries move down by one
    }
    self.push(t, bps);
}
```

`remove` and `push` update `len` and leave the unused entries zeroed.
`schedule` is only called with `t` equal to the current epoch plus two, so
`t` is never less than the last entry's `epoch`.

A stored schedule is valid only if `len` is between 1 and 3, the entries
in use are in strictly increasing `epoch` order and the unused entries are
zero. A v5 state with an invalid schedule MUST fail to deserialize.

The **commission of a vote state for epoch `E`** is:

- for a v5 state, the `bps` of `effective(E)` on the schedule of the
  commission kind in question, or the `bps` of entry 0 if `effective(E)`
  returns nothing;
- for a v4 state, the stored rate, and for an older state whatever the
  proposal that uses the rate specifies today.

### Vote Program

All vote instructions MUST support deserializing v5 vote accounts. All
instructions besides `InitializeAccount` and `InitializeAccountV2` MUST
process the vote account in the following order:

1. Deserialize versioned vote state.
2. Check for initialization or return
   `InstructionError::UninitializedAccount`.
3. Convert to v4 as specified in [SIMD-0185] if the state is older than
   v4.
4. Convert to v5: each schedule gets the single entry
   `(current_epoch + 2, bps)`, where `bps` is the v4 field the schedule
   replaces and `current_epoch` is the epoch of the `Clock` sysvar.
   `location` is `(0, 0, 0)`, `votes` is dropped and all other fields are
   copied.

The account is stored as v5 exactly when the instruction succeeds and
stores vote state. That includes a commission update that does not change
the rate. An instruction that fails stores nothing. A successful
instruction that does not store vote state leaves the account data as it
is, except that a `Withdraw` of the full balance zeroes it as specified in
[SIMD-0185]. Where an instruction checks the vote state version, the check
applies to the stored version, before any conversion. When an account is
stored as v5, all account data after the serialized state MUST be zeroed.

Conversion is lazy. A vote account stays on its stored version until a
Vote program instruction stores it. This proposal adds no conversion at
the epoch boundary.

Instruction changes:

- **`InitializeAccount`, `InitializeAccountV2`** MUST create v5 accounts.
  Each schedule gets the single entry `(current_epoch + 2, bps)`, where
  `bps` is the value the instruction would have stored in the v4 field
  that schedule replaces. `location` is `(0, 0, 0)`. The account size
  stays `3762` bytes.
- **`UpdateCommission`, `UpdateCommissionBps`**: after the existing
  validation, the Vote program MUST apply
  `schedule(current_epoch + 2, bps)` to the schedule of the commission
  kind being updated, where `bps` is the value it would have stored in
  the v4 field. The new rate takes effect in epoch `current_epoch + 2`.
  An existing entry for that epoch is overwritten, and entries for
  earlier epochs are left as they are. If `current_epoch + 2` overflows,
  the instruction MUST fail with `InstructionError::InvalidAccountData`.
- **`DepositDelegatorRewards`** ([SIMD-0123]) MUST fail unless the stored
  state version is 4 or 5. It stores vote state, so a v4 account it
  succeeds on is stored as v5.
- **Tower vote instructions** (`Vote`, `VoteSwitch`, `UpdateVoteState`,
  `UpdateVoteStateSwitch`, `CompactUpdateVoteState`,
  `CompactUpdateVoteStateSwitch`, `TowerSync`, `TowerSyncSwitch`) have
  nothing to write to in v5. They MUST fail with
  `InstructionError::InvalidInstructionData` and MUST NOT modify the vote
  account.
- All other instructions keep their v4 behavior. The instruction that
  sets `location` is specified in [SIMD-0674].

### Runtime

**Inflation rewards.** Rewards earned in epoch `E` are calculated at the
beginning of epoch `E + 1` from the vote account state at that point,
which is the state `epoch_credits` and the commission collector are read
from ([SIMD-0232]). The commission MUST be resolved as follows:

1. If that state is v5 and `inflation_rewards_commission.effective(E)`
   returns an entry, use that entry's `bps`.
2. Otherwise select a vote state as specified in [SIMD-0249] and use the
   commission of that state for epoch `E`.

A node that recalculates partitioned rewards after booting from a
snapshot ([SIMD-0118]) resolves the commission from the same states.

**Block revenue.** [SIMD-0123] takes the commission for a block produced
in epoch `E` from the vote account state at the beginning of epoch
`E - 1`. That selection is unchanged. The rate MUST be the commission of
that state for epoch `E`.

**Unchanged results.** An update made in epoch `X` is first visible in
the vote account state at the beginning of epoch `X + 1`, which is the
state [SIMD-0249] and [SIMD-0123] use for epoch `X + 2`, and `X + 2` is
the effective epoch the Vote program records for it. The rate resolved
above is therefore the rate those proposals select today, for every
account and every epoch. It is used exactly as before, including any cap.

Step 1 always applies to an account that was already v5 at the beginning
of epoch `E - 1` and has not been closed since. Only step 2 needs state
from an earlier epoch boundary.

Where the runtime itself writes vote state, as [SIMD-0123] does for
`pending_delegator_rewards`, it MUST keep the stored version. The stakes
cache, the epoch stakes in snapshots and the Stake program MUST support
v5 vote accounts.

### Edge Cases

- **Several updates in one epoch** overwrite the same entry, so the
  schedule does not grow. An update that does not change the last
  entry's rate adds nothing.
- **Full schedule.** Pushing a new entry drops the oldest one. It is
  never one that step 1 needs.
- **Converted and new accounts.** Until the first entry takes effect, the
  rate is the one [SIMD-0249] selects today. The first entry's `epoch` is
  an upper bound: its rate may already be in force.
- **Closed and re-created accounts** start with a fresh schedule. The
  rates for the epoch of creation and the one after can still come from
  the old account's state, which the new account does not show.
- **Reading.** For a reader at epoch `C`, `effective(C)` is the rate in
  force and entries after `C` are pending. There are at most two.

### Validator Components Affected

| Validator Component             | Impact                                 |
|---------------------------------|----------------------------------------|
| Transaction Execution (Runtime) | Commission lookup; writes keep version |
| Virtual Machine                 | None                                   |
| Block Packing                   | None                                   |
| Consensus                       | Reward amounts unchanged; new layout   |
| Gossip                          | None                                   |
| Turbine                         | None                                   |
| Snapshots                       | Stakes cache must support v5           |
| On-Chain Core BPF Programs      | Vote program; Stake program reads v5   |
| Other (RPC)                     | Parsed vote account, `getVoteAccounts` |

## Alternatives Considered

- **A runtime-written commission history appended to v4.** The first
  version of this proposal. It made the runtime rewrite every vote
  account every epoch.
- **A different schedule size.** An update made in epoch `X` applies
  from `X + 2`, so the rate in force and two pending rates can exist at
  once. With two entries, a validator that updates in two consecutive
  epochs holds only pending entries and the rate in force is no longer in
  the account. Three is the smallest size at which it always is. A fourth
  entry would keep the previous epoch's rate readable, but that is history
  the protocol does not need.
- **Seeding a converted account at `current_epoch + 1`.** The stored v4
  value already applies from then, so the rate would be readable one
  epoch sooner with the same rewards. A new account cannot use that seed,
  because closing and re-creating it would bring a rate forward, and one
  rule for both is simpler.
- **Removing more fields, or keeping `votes`.** `root_slot` and
  `last_timestamp` are kept because no dependency says they are unused.
  Keeping `votes` would allow activation before the Alpenglow migration,
  at the cost of another version for the cleanup.

## Impact

- **Stake delegators and stake pools** can read the rate in force and the
  pending rates from the vote account, on chain or off chain, once the
  account has been v5 for two epochs.
- **Validators** set commission as before, with the same two-epoch delay.
  An account is converted the first time a Vote program instruction
  stores it.
- **Programs and tools that parse vote accounts** MUST add support for
  v5. The offsets of all fields after `block_revenue_collector` change.

## Security Considerations

- A commission change cannot take effect earlier than under [SIMD-0249].
  The Vote program only writes entries with an effective epoch of the
  current epoch plus two and never modifies an earlier one.
- Closing and re-creating a vote account can hide a pending change from
  someone who reads only the account, for at most two epochs. The same is
  true today.
- All clients must resolve the same rate. Both steps are functions of
  vote account state at epoch boundaries, and where both apply they
  select the same rate.

## Drawbacks

- Everything that parses vote accounts has to be updated.
- Clients cannot drop the earlier epoch-boundary state while v4 accounts
  exist, and conversion is lazy.
- Activation is tied to the Alpenglow migration because `votes` is
  removed.

## Backwards Compatibility

- A feature gate activates the Vote program and runtime changes at the
  same epoch boundary.
- The feature MUST NOT be activated before the features for [SIMD-0185]
  and [SIMD-0249], or before the `migration success` account defined in
  [SIMD-0384] contains a valid certificate.
- The rules that refer to [SIMD-0123] apply once its feature is active.
- Accounts on v4 or older remain valid and are read as before.
- Reward amounts do not change at activation or afterwards.

## Conformance

Test vectors for the Vote program and a local ledger will cover:
conversion of a v4 account, several updates in one epoch, updates in four
consecutive epochs, a snapshot boot during reward
distribution, closing and re-creating an account, a runtime write to a v4
account, and `DepositDelegatorRewards` on v3, v4 and v5 accounts. Clients
compare bank hashes across the ledger.
