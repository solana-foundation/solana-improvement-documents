---
simd: '0676'
title: On-Chain Epoch Stakes
authors:
  - sam0x17
category: Standard
type: Core
status: Review
created: 2026-03-23
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Publish the stake and vote-account snapshot taken at the start of epoch `E` in
an on-chain account keyed by `E`. It contains delegated stake, identity,
commission, accumulated rewards, latest vote, root slot, and the BLS key and
rank used for Alpenglow in `E + 1`.
Retain eight snapshots: the current epoch and seven previous epochs. Close
older accounts at epoch boundaries.

## Motivation

Historical epoch stakes currently live in validator-local caches and snapshot
manifests. Publishing per-validator stake weights, identities, and BLS keys and
ranks as account data puts these consensus inputs under the bank hash.
Rebuilding stake-weight and BLS-rank lookups from these accounts during snapshot
loading is an intended follow-up, reducing reliance on historical vote-account
data in the manifest.

On-chain programs can inspect stake distributions, and read-layer clients can
map BLS certificate ranks to validators. Existing account reads and subscriptions
can serve this data without a new RPC method or Geyser interface.

Removing caches or snapshot-manifest fields remains follow-up work. This
proposal does not change snapshot loading, reward calculation, or existing RPC
methods.

## New Terminology

An **epoch stakes account** stores the snapshot taken at the start of its named
epoch. It is a runtime-managed account with a dedicated owner address and a
program-derived address (PDA) keyed by that epoch.

## Detailed Design

### Snapshot Timing

At the boundary into epoch `E`, take the snapshot after applying feature
activations, calculating stake activation and deactivation for `E`, and applying
the validator admission rules used to form the consensus stake set for `E + 1`.
Use vote-account state before distributing inflation rewards at this boundary
and before processing any transactions in the new epoch. The consensus stake
set for `E + 1` is derived from this vote-account snapshot.

Include exactly the vote accounts selected under [SIMD-0357] at the boundary
into `E`, from which VAT is deducted for admission to consensus in `E + 1`.
An account admitted at the previous boundary is included only if it is selected
again at this boundary. Do not add previous-epoch VAT payers solely for reward
calculation.

Publish that snapshot as `epoch_stakes(E)`, with `E` in its header. The snapshot
is immutable until closure. `delegated_stake` and `total_stake` describe this
snapshot, not live stake later in the epoch: partitioned inflation rewards can
subsequently increase delegated balances.

The key is the snapshot epoch. The stake set and Alpenglow ranks in
`epoch_stakes(E)` apply to consensus in `E + 1`.

For the same vote-account incarnation present in consecutive snapshots, let
`lamports(E)` be its `accumulated_lamports` in `epoch_stakes(E)`. When both
snapshots are within Alpenglow and the counter remains continuous and does not
saturate:

| Value | Epoch it describes |
|-------|--------------------|
| Stake set and Alpenglow ranks in `epoch_stakes(E)` | Consensus in `E + 1` |
| Inflation commission in `epoch_stakes(E)` | Rewards in `E + 1` |
| `lamports(E) - lamports(E - 1)` | Reward lamports recorded during `E - 1` |
| `latest_vote` and `root_slot` | Vote-account state at the start of `E` |

For example, `epoch_stakes(100)` contains the snapshot taken at the start of
100. Its inflation commission applies to rewards earned in 101. Under the
conditions above, its accumulated lamports minus those in `epoch_stakes(99)`
give the reward lamports recorded during 99.

`latest_vote` and `root_slot` preserve the historical health indicators from
that same boundary snapshot. They are not live values and are not refreshed
during the epoch. A missing value is distinct from slot zero.

### Account Structure

Version 1 uses a fixed binary layout. All multi-byte integers are little-endian.
The header is 32 bytes, followed by `num_entries` entries of 160 bytes each.
Entries are sorted in ascending lexicographic order of `vote_pubkey` bytes.

| Header field | Type | Byte offset |
|--------------|------|-------------|
| `version` | `u32` | 0 |
| `num_entries` | `u32` | 4 |
| `epoch` | `u64` | 8 |
| `total_stake` | `u64` | 16 |
| `_reserved` | `[u8; 8]` | 24 |

`version` is 1. `epoch` is the snapshot epoch used to derive the address.
`total_stake` is the sum of `delegated_stake` across all entries. Reserved bytes
**MUST** be zero.

| Entry field | Type | Byte offset |
|-------------|------|-------------|
| `vote_pubkey` | `Pubkey` | 0 |
| `node_pubkey` | `Pubkey` | 32 |
| `delegated_stake` | `u64` | 64 |
| `accumulated_lamports` | `u64` | 72 |
| `inflation_rewards_commission_bps` | `u16` | 80 |
| `block_revenue_commission_bps` | `u16` | 82 |
| `alpenglow_rank` | `u16` | 84 |
| `_reserved` | `[u8; 10]` | 86 |
| `bls_pubkey_compressed` | `[u8; 48]` | 96 |
| `latest_vote` | `u64`, `u64::MAX` = none | 144 |
| `root_slot` | `u64`, `u64::MAX` = none | 152 |

Each vote account selected at the boundary into `E` appears once. This snapshot can
include entries excluded from the BLS rank map because they share a validator
identity or BLS key. Those entries retain their stake and commission data and
use `u16::MAX` for the rank. The fields contain:

- `vote_pubkey`: the vote account address.
- `node_pubkey`: its validator identity at the snapshot.
- `delegated_stake`: the effective delegated stake in lamports selected at the
  snapshot. It is fixed even if subsequent reward payouts increase live stake.
- `accumulated_lamports`: the vote state's cumulative reward counter at the
  snapshot, or zero when no counter has been recorded. For vote-state formats
  with `epoch_credits`, use the last entry's cumulative `credits` value, or zero
  if that history is empty. Preserve the value without subtracting a migration
  offset or converting it to a per-epoch amount.
- `inflation_rewards_commission_bps` and `block_revenue_commission_bps`: the raw
  vote-account commission rates in basis points. Every `u16` value is valid,
  including values above 10,000. Publication **MUST NOT** clamp them. Reward
  calculation clamps to 10,000 when applying a rate. For vote-state formats
  with only a legacy inflation commission percentage, publish that percentage
  multiplied by 100 and a block-revenue commission of 10,000, following
  [SIMD-0185].
- `alpenglow_rank`: the rank in the canonical Alpenglow BLS validator set for
  `E + 1`, including its BLS-key and identity deduplication. Rank zero has the
  highest eligible stake. Write `u16::MAX` for an entry outside that active set.
- `bls_pubkey_compressed`: the vote state's 48-byte compressed BLS public key,
  or 48 zero bytes if it has no BLS key.
- `latest_vote`: the last voted slot recorded in the snapshot's vote state,
  or `u64::MAX` when absent.
- `root_slot`: the root slot recorded in that vote state, or `u64::MAX` when absent.
- `_reserved`: 10 zero bytes. Reusing them requires a later protocol change
  that preserves existing readers' interpretation of version 1.

Collector addresses are excluded. Changes to which vote-account state supplies
reward collectors belong to a separate proposal. This account does not define
or change those rules.

The admission limit from [SIMD-0357] bounds each account to 2,000 entries.
The data length **MUST** be exactly `32 + num_entries * 160` bytes, at most
320,032 bytes (about 313 KiB). Eight retained accounts occupy at most
2,560,256 bytes (about 2.44 MiB), excluding account and storage metadata.

### Account Addresses and Owner

For the snapshot taken at the start of `E`:

```text
epoch_stakes(E) = PDA(owner_id, [b"epoch_stakes", E.to_le_bytes()])
```

`E` is encoded as eight little-endian bytes. Each epoch has a distinct address.
Its data is never rotated to another address or republished after closure.

The owner is the following vanity address:

```text
EpochStakes11111111111111111111111111111111
```

No program is deployed at this address. Feature activation **MUST NOT** create,
fund, or modify an account at the owner address. If it is already funded, it
remains a non-executable, system-owned account with no data. The runtime funds
the per-epoch PDAs as specified below.

Only the runtime creates and closes the epoch stakes accounts. Transactions
cannot modify their data or owner, or debit their lamports.

Neither the owner address nor the per-epoch PDAs are added to the reserved
account keys. Ordinary account-lock rules apply. Transactions can fund these
addresses without gaining control of them.

### Publication and Funding

On the first block of each epoch `E` after activation, before processing any
transactions, the runtime **MUST**:

1. Publish the snapshot defined under [Snapshot Timing](#snapshot-timing) at
   `epoch_stakes(E)`.
2. Preserve any existing lamports at the PDA being published. Set its balance
   to the greater of its existing balance and the current rent-exempt minimum
   for the serialized data. Increase capitalization only by the shortfall
   supplied by the runtime.
3. Close expired accounts as specified below.

An over-funded PDA keeps its full balance. Publication does not burn the excess
or reduce the balance to the rent-exempt minimum. Repeating publication for an
already-published epoch leaves both its data and balance unchanged. Transfers
to a retained account can increase its balance without changing the snapshot.

### Activation, Retention, and Closure

Activation **MUST** occur at an epoch boundary with [SIMD-0326] and [SIMD-0357]
active. The rank and entry-count bound depend on those proposals.

Umar Bhatty of [Wire Outpost] asked for [at least five epochs] of history for
on-chain delegation scoring. Eight snapshots leave room for delayed reads.
At the 2,000-entry limit, it bounds live account data at about 2.44 MiB.

[Wire Outpost]: https://hub.wire.network/outpost
[at least five epochs]: https://github.com/solana-foundation/solana-improvement-documents/pull/676#issuecomment-5904637000

Let `A` be the first epoch boundary at which this feature publishes snapshots.
Publish only `epoch_stakes(A)`. No earlier snapshot is backfilled, and no future
snapshot epoch is published. The snapshot for consensus in `A` is therefore
unavailable through these accounts during the activation epoch. The first
published snapshot describes consensus in `A + 1`.
After processing the boundary into epoch `E`, the retained range is:

```text
max(A, E.saturating_sub(7)) <= snapshot_epoch <= E
```

This retains at most eight accounts: current and seven previous epochs. The
window fills over subsequent boundaries. If an epoch has no blocks, no snapshot
is taken for that epoch, so its account remains absent. Gaps do not extend the
lower bound, and a jump across multiple epochs still closes every previously
published account below it.

At each boundary, the runtime **MUST** clear the data and zero the lamport
balance of every expired epoch stakes account, reducing capitalization by its
full balance. The burn includes lamports transferred to the account by users.
Closure **MUST** affect only accounts owned by the epoch stakes owner address.

System-owned placeholders are left unchanged. These can exist before first
publication, in skipped epochs, or at expired addresses that users fund again
after closure. The owner check therefore remains necessary after the initial
eight epochs. Such placeholders do not count as published snapshots and are
never backfilled outside the retained range.

Consumers can authenticate a snapshot by checking the derived address, owner,
format version, header epoch, and data length. A funded PDA alone does not prove
publication. Older history requires an archive. Neither missing snapshots nor
missing entries imply zero stake, accumulated rewards, or commission.

## Alternatives Considered

### Sysvars

Making these accounts sysvars would require defining how a family of
per-epoch addresses interacts with sysvar access such as `sol_get_sysvar` and
write-lock demotion. A dedicated owner lets programs read them as ordinary
account data, without adding sysvar access or reserved-key rules.

### Publishing the Leader Schedule

[SIMD-0558] provides a separate interface for current and next leader lookup.
This proposal exposes its underlying stake data and Alpenglow ranks. The
interfaces share no account or ABI. Schedule derivation uses vote-account keys
under [SIMD-0180]. The `node_pubkey` here is snapshot metadata.

### Rolling or Combined Accounts

Fixed previous/current/next addresses require copying data and change what an
address means at a boundary. A combined history account also requires rewriting
retained data. Epoch-keyed accounts have immutable contents and independent
addresses within the retention window.

### Geyser Notifications Alone

Notifications alone do not give a reconnecting or newly started consumer the
existing snapshots. Account state supports both reads and subscriptions through
existing interfaces, including Geyser.

## Impact

Validators publish one account per epoch and close expired accounts. The
2,000-entry limit and eight-epoch window bound live data at about 2.44 MiB.
Existing RPC methods and validator configuration remain unchanged.

Programs can read the accounts directly. Indexers can use `accountSubscribe`,
`programSubscribe`, or Geyser account notifications and archive snapshots before
expiry. Consumers mapping certificates to ranks use the preceding epoch's
snapshot. Consumers of commissions and accumulated rewards use the timing
rules above.

## Security Considerations

Account data is covered by the bank hash. Publication uses the existing
consensus snapshot and canonical rank map. It does not alter stake selection,
reward calculation, or snapshot loading.

Publication preserves pre-funded balances and mints only a rent shortfall.
Closure burns the full expired balance. Funding uses the current rent minimum.
At 5,080 lamports per byte and 128 bytes of account overhead, eight maximum
accounts need about 13.01 SOL if entirely funded by the runtime. Once the window
is full, an expired account offsets funding a new account of the same size.
Transfers can raise balances but cannot enlarge the account data.

## Future Work

Snapshot-manifest validation or replacement, per-epoch reward totals, and
changes to collector lookup remain separate proposals. This format omits
authorized voters, collector addresses, and the full reward-counter history, so it
cannot replace the complete cached vote-account state. The pre-reward stake
snapshot also does not replace the stake balances used for reward recalculation.

## Backwards Compatibility

This introduces a new account format and owner address under a feature gate.
It changes no existing account layout, sysvar, RPC method, or reward rule.
All validators must implement publication, funding, and closure before the
feature activates.

[SIMD-0180]: ./0180-vote-account-leader-schedule.md
[SIMD-0185]: ./0185-vote-account-v4.md
[SIMD-0326]: ./0326-alpenglow.md
[SIMD-0357]: ./0357-alpenglow_validator_admission_ticket.md
[SIMD-0558]: ./0558-leader-info-syscall.md
