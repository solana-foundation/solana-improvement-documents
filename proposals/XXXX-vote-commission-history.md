---
simd: 'XXXX'
title: Vote Account Commission History
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

Add a bounded `commission_history` field to the vote account state. At every
epoch boundary the runtime appends the inflation-rewards and block-revenue
commission rates that apply to the epoch that just started, keeping the
most recent `MAX_COMMISSION_HISTORY = 5` entries. The rates are the ones the
runtime itself resolves under SIMD-0249 and SIMD-0123, so the entry for
epoch `E` is exactly what delegators to that vote account are charged for
epoch `E`.

## Motivation

SIMD-0249 (activated on mainnet-beta at epoch 991) delays every commission
update by a full epoch: inflation rewards for epoch `E` are calculated at
the start of `E + 1` using the commission from the vote account state as it
existed at the beginning of `E - 1`. SIMD-0123 applies the same rule to the
block-revenue commission and collector: they are read from the vote account
state at the beginning of the previous epoch.

This removed the end-of-epoch commission rug, but it changed what the vote
account means. `inflation_rewards_commission_bps` and
`block_revenue_commission_bps` no longer describe the rates being charged.
They describe the rates that will be charged two boundaries from now if the
validator does not change them again. The rates that actually apply to the
current epoch, and to the previous ones, are held only in each validator's
in-memory epoch-stakes snapshots. They are not readable from any account,
sysvar or syscall.

Consequences today:

- An on-chain program cannot learn the effective commission of a vote
  account. Stake pools, delegation strategies and liquid-staking scorers
  select validators on commission, and the only value they can read is the
  pending one.
- An off-chain reader can only reconstruct the effective rate if it
  snapshotted the account at each of the last two epoch boundaries and
  never missed one. Delegators are dependent on indexers for a fact the
  runtime already computed.
- A validator can still play the delay window: raise, hold for one epoch,
  lower. Each individual observation of the account looks reasonable; only
  the sequence shows the pattern. Nothing on chain keeps the sequence.

Every stake pool carries some private approximation of this history. For
example, the delegation scorer we operate keeps a ten-entry ring buffer of
observed commissions per validator, updated by a permissionless crank that
must run every epoch, and takes the maximum observed within an epoch as a
guard against changes it did not see. It is ~100 lines of program code plus
an off-chain process whose only job is to observe a value the runtime
already knows and could have written down.

Expected outcome: one account read gives the rate in force, the pending
rate, and enough history to evaluate the validator's recent behaviour. The
information is written by the runtime, so it cannot be forged by the
validator, and it lands in the same boundary bank that already rewrites vote
accounts to credit inflation rewards.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0185]: Vote Account V4**

    Provides `inflation_rewards_commission_bps` and
    `block_revenue_commission_bps` and the fixed 3762-byte account size
    with the headroom this proposal uses.

- **[SIMD-0249]: Delay Commission Updates**

    Defines which epoch's vote state the inflation-rewards commission is
    read from. This proposal records the result of that rule.

- **[SIMD-0123]: Block Revenue Sharing**

    Defines which epoch's vote state the block-revenue commission is read
    from. Same treatment.

[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/pull/185
[SIMD-0249]: https://github.com/solana-foundation/solana-improvement-documents/pull/249
[SIMD-0123]: https://github.com/solana-foundation/solana-improvement-documents/pull/123

## New Terminology

- **Effective commission for epoch `E`**: the commission rates the runtime
  applies when computing rewards earned in epoch `E`. Under SIMD-0249 and
  SIMD-0123 these are the rates in the vote account state at the beginning
  of epoch `E - 1`, with the fallbacks those proposals define.
- **Commission snapshot**: one `(epoch, inflation_rewards_commission_bps,
  block_revenue_commission_bps)` entry in `commission_history`.

## Detailed Design

### State

A new field is appended to `VoteStateV4` after `last_timestamp`:

```rust
pub const MAX_COMMISSION_HISTORY: usize = 5;

pub struct CommissionSnapshot {
    /// Epoch these rates apply to.
    pub epoch: u64,
    /// Inflation-rewards commission applied to rewards earned in `epoch`.
    pub inflation_rewards_commission_bps: u16,
    /// Block-revenue commission applied to block revenue earned in `epoch`.
    pub block_revenue_commission_bps: u16,
}

pub struct VoteStateV4 {
    // ... existing fields unchanged, through `last_timestamp` ...
    /// Effective commission for the most recent epochs, oldest first.
    /// Written only by the runtime at epoch boundaries.
    pub commission_history: Vec<CommissionSnapshot>,
}
```

Serialization follows the existing vote state encoding: a `u64` length
prefix followed by fixed-width little-endian entries of 12 bytes each.
Maximum serialized size of the field is `8 + 5 * 12 = 68` bytes. Entries
are strictly increasing in `epoch` and `commission_history.len() <=
MAX_COMMISSION_HISTORY` at all times.

The required vote account size of 3762 bytes MUST remain unchanged. V4
removed `prior_voters` (1545 bytes) and `commission: u8`, and added 125
bytes of new fields, so the maximum serialized V4 state is 2341 bytes and
the new field fits with room to spare.

### Runtime

In the first bank of each epoch `X`, in the same epoch-boundary processing
that computes and credits inflation rewards to vote accounts, the runtime
MUST, for every vote account present in the epoch-stakes vote account set
for epoch `X`:

1. Resolve the effective commission for epoch `X`:
   - if the vote account existed in the epoch-stakes snapshot taken at the
     beginning of epoch `X - 1`, take `inflation_rewards_commission_bps`
     and `block_revenue_commission_bps` from that snapshot;
   - otherwise take them from the snapshot taken at the beginning of
     epoch `X`.

   This is the same resolution SIMD-0249 and SIMD-0123 perform for the
   rewards of epoch `X` one boundary later, so the recorded rates and the
   applied rates are identical by construction. SIMD-0249's third fallback
   (state at the beginning of `X + 1`) covers only accounts that were absent
   from both snapshots; such accounts are not in the set iterated here and
   receive no entry for `X`.

2. If `commission_history` is non-empty and its last entry has
   `epoch >= X`, do nothing (idempotent).

3. Otherwise append `CommissionSnapshot { epoch: X, ... }` and, while
   `len() > MAX_COMMISSION_HISTORY`, remove the oldest entry.

4. Store the account.

The snapshot for the beginning of `X - 1` is already retained by the bank
for SIMD-0249 (it is the `distribution_epoch_vote_accounts` set that will
price the rewards of epoch `X`), so no additional snapshot retention is
required.

The write MUST happen before any partitioned stake-reward distribution for
the epoch and MUST be reflected in the bank hash of the boundary bank.

### Vote program

The vote program MUST NOT modify `commission_history` in any instruction.
`UpdateCommission`, `UpdateCommissionBps` and any collector or authority
update leave it untouched. `InitializeAccount` / `InitializeAccountV2` set
it to empty. Deserializers in the vote program MUST accept the field.

### Reading

- On chain: any program that already deserializes the vote account (stake
  pools, delegation strategies) reads `commission_history.last()` for the
  rates in force in the current epoch, compares it with the live
  `*_commission_bps` fields to see whether a change is pending, and scans
  the vector for recent movement.
- Off chain: `getAccountInfo` with `jsonParsed` encoding SHOULD expose the
  field as `commissionHistory: [{epoch, inflationRewardsCommissionBps,
  blockRevenueCommissionBps}]`. `getVoteAccounts` MAY add the same array.
  Both are RPC conveniences and not part of consensus.

### Edge Cases

- **Activation.** At the first boundary after activation the runtime has
  both prior snapshots (they already exist for SIMD-0249), so the first
  entry is written immediately and `len()` is 1. Readers MUST handle
  `len() < MAX_COMMISSION_HISTORY`.
- **New vote account.** An account created during epoch `X` is absent from
  the boundary set at `X` and gets its first entry at the boundary of
  `X + 1`, using the state at the beginning of `X + 1`. This matches the
  SIMD-0249 fallback for that account's first rewarded epoch.
- **Closed and re-created account.** Closing a vote account removes the
  history with the account. Re-initializing at the same address starts
  empty.
- **Unstaked vote accounts.** They are in the boundary set and receive
  entries like any other. They have no delegators, so the cost is the
  account write only. Implementations MAY skip vote accounts with zero
  delegated stake in both snapshots; the resulting history is then sparse
  for those accounts, and readers MUST NOT assume consecutive epochs.
- **Undersized accounts.** All V4 accounts are 3762 bytes per SIMD-0185.
  If an account's data is somehow too short to hold the appended field the
  runtime MUST skip the append rather than fail the boundary.
- **Replay and reprocessing.** Step 2 makes the append idempotent for a
  given epoch.
- **Legacy vote state versions (V0 to V3).** Not written. Accounts are
  converted to V4 on first write by the vote program (SIMD-0185); the
  runtime MUST NOT convert them at the boundary for this purpose.

### Validator Components Affected

| Validator Component             | Impact                                     |
|---------------------------------|--------------------------------------------|
| Transaction Execution (Runtime) | Boundary append + store per vote account   |
| Virtual Machine                 | None                                       |
| Block Packing                   | None                                       |
| Consensus                       | Boundary bank hash includes the writes     |
| Gossip                          | None                                       |
| Turbine                         | None                                       |
| Snapshots                       | In account data; no format change          |
| On-Chain Core BPF Programs      | Vote program accepts field, never writes   |
| Other (RPC)                     | Optional `jsonParsed` exposure             |

## Alternatives Considered

- **On-Chain Epoch Stakes (SIMD-0511, PR #676).** Publishes one PDA per
  epoch with a 224-byte entry per vote account, sorted by vote pubkey,
  including both commission rates in bps. As drafted, the account keyed `E`
  carries the vote state from the beginning of `E - 1`, which is the state
  SIMD-0249 prices epoch `E` with, so it exposes the same fact and more.
  The differences are timing and cost. SIMD-0511 cannot activate before
  Alpenglow (SIMD-0326, SIMD-0357), is bounded to the 2,000-validator
  admitted set, and needs one ~438 KiB account loaded per epoch of
  history. This proposal is 60 bytes in an account the consumer already
  loads, has no Alpenglow dependency and covers every vote account. The
  two are compatible: this field is the interim, and once SIMD-0511 is
  active with a retention window of at least five epochs it is redundant
  for the admitted validator set.
- **A syscall, `sol_get_epoch_commission(vote_pubkey)`, in the style of
  SIMD-0133.** No state change, but it serves on-chain programs only.
  Wallets, explorers, stake pool front ends and delegators would still be
  blind, it can only answer for the current epoch, and it adds a syscall
  for a value that is cheaper to write once than to serve on demand.
- **A single `effective_commission_bps` (or `next_epoch_commission`) field**,
  as suggested in discussion #254. Four bytes, but it shows only the
  current rate. It cannot show a raise that was reverted one epoch later,
  which is the residual manipulation SIMD-0249 leaves open. Five entries
  cost 56 bytes more and close that gap.
- **Vote-program-side append on the first vote of a new epoch**, the way
  `epoch_credits` is maintained. Rejected: it would record the stored field
  at whatever slot the first vote landed, not the effective rate, and
  validators that do not vote would get no entry.
- **Off-chain indexers (status quo).** Every consumer builds its own
  boundary sampler, a missed epoch is a hole, and delegators trust a third
  party for a value the runtime computed and discarded.

### Why 5

Under SIMD-0249 a change made in epoch `X` first applies in epoch `X + 2`.
Five entries show `X - 4 .. X`, i.e. a little under a week at today's slot
times: enough to see one full raise-hold-revert cycle across the delay
window and one complete deactivate-and-redelegate cycle either side of it.
The constant is not load-bearing. `epoch_credits` keeps 64 entries; 64
snapshots (776 bytes) would also fit in the existing account size.

## Impact

- **Delegators and stake pools** get the effective commission and recent
  history from the vote account itself, with no indexer dependency, and
  can delete their private ring-buffer approximations.
- **Validators** see no change to how they set commission. The history is
  runtime-written and cannot be edited by the vote authority.
- **Core contributors** add one bounded append in the epoch-boundary path,
  next to code that already loads the same snapshots and stores the same
  accounts.
- **RPC and explorers** can show "in force / pending / last N" from a
  single account read.

## Security Considerations

- The field is written only by the runtime and never by the vote program,
  so a validator cannot forge or erase its own history short of closing
  the account, which also removes its stake and credits history.
- All clients must produce byte-identical results because the boundary
  bank hash covers the writes. The resolution rule is the one SIMD-0249
  already requires clients to agree on; this proposal adds no new source of
  divergence beyond the append itself.
- No new authority, instruction or lamport flow is introduced. Rent is
  unaffected because the account size does not change.

## Drawbacks

- Roughly one extra account store per vote account per epoch for accounts
  that receive no inflation reward that epoch (staked accounts are already
  rewritten in the same bank). On mainnet this is on the order of a few
  thousand account writes per epoch.
- Every V4 deserializer that is strict about trailing bytes needs an
  update. See Backwards Compatibility.

## Backwards Compatibility

- The field is appended at the end of the V4 layout, so offsets of all
  existing fields are unchanged.
- Existing V4 accounts are stored in fixed 3762-byte buffers padded with
  zeros. The appended `Vec` therefore decodes as a zero-length vector on
  every existing account with no migration.
- Readers that walk the V4 layout with a cursor and ignore trailing bytes
  continue to work unchanged. Readers that reject trailing bytes must be
  updated before activation. If maintainers prefer a hard version bump for
  layout changes, the same field can ship as `VoteStateV5` with the
  conversion rules of SIMD-0185; the runtime and vote-program behaviour
  above is unchanged either way.
- A feature gate activates the runtime append and the vote program's
  acceptance of the field at the same epoch boundary.

## Conformance

The change will be accompanied by a localnet ledger that:

1. runs at least two epochs before activation, showing `commission_history`
   absent / empty;
2. activates the feature and shows one entry per vote account at the first
   boundary;
3. issues `UpdateCommissionBps` during epoch `A + 1` and shows that the
   entry for `A + 2` still carries the old rate and the entry for `A + 3`
   carries the new one, matching the rates the runtime used for the
   corresponding inflation rewards;
4. runs past `A + 5` and shows the vector capped at 5 with the oldest entry
   evicted;
5. creates a vote account mid-epoch and shows its first entry at the next
   boundary.

Clients compare bank hashes across the ledger.
