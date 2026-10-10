---
simd: '0511'
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
commission, credits, and the BLS key and rank used for Alpenglow in `E + 1`.
Retain eight snapshots: the current epoch and seven previous epochs. Close
older accounts at epoch boundaries.

## Motivation

Epoch stakes are inputs to consensus, but their snapshots currently live in
validator-local caches and snapshot manifests. Publishing them as account data
lets on-chain programs inspect stake distributions and lets read-layer clients
map BLS certificate ranks to validators. Existing account reads and subscriptions
can serve this data without a new RPC method or Geyser interface.

The account data is covered by the bank hash. A later proposal can use it to
validate or replace snapshot-manifest fields. This proposal does not change
snapshot loading, reward calculation, or existing RPC methods.

## New Terminology

An **epoch stakes account** stores the snapshot taken at the start of its named
epoch. It is a runtime-managed account owned by the Epoch Stakes program, with a
program-derived address (PDA) keyed by that epoch.

## Detailed Design

### Snapshot Timing

At the boundary into epoch `E`, take the snapshot after applying feature
activations, calculating stake activation and deactivation for `E`, and applying
the validator admission rules used to form the consensus stake set for `E + 1`.
Use vote-account state before distributing inflation rewards at this boundary
and before processing any transactions in the new epoch. These are the same
stake and vote-account values selected for consensus in `E + 1`.

Publish that snapshot as `epoch_stakes(E)`, with `E` in its header. The snapshot
is immutable until closure. `delegated_stake` and `total_stake` describe this
snapshot, not live stake later in the epoch: partitioned inflation rewards can
subsequently increase delegated balances.

The key is the snapshot epoch. The stake set and Alpenglow ranks in
`epoch_stakes(E)` apply to consensus in `E + 1`. To interpret a certificate for
epoch `E`, use `epoch_stakes(E - 1)`.

For a vote account present in consecutive snapshots with a continuous credits
counter, let `credits(E)` be its `cumulative_credits` in `epoch_stakes(E)`:

| Value | Epoch it describes |
|-------|--------------------|
| Stake set and Alpenglow ranks in `epoch_stakes(E)` | Consensus in `E + 1` |
| Inflation commission in `epoch_stakes(E)` | Rewards in `E + 1` |
| `credits(E) - credits(E - 1)` | Credits recorded during `E - 1` |

For example, `epoch_stakes(100)` contains the snapshot taken at the start of
100. Its inflation commission applies to rewards earned in 101. Its credits
minus those in `epoch_stakes(99)` give the credits recorded during 99.

Under [SIMD-0249], rewards earned in `R` use the inflation commission from
`epoch_stakes(R - 1)`. If the vote account was absent from that snapshot, the
reward rules fall back to its state at the start of `R`, then `R + 1`.
The corresponding published snapshots are `epoch_stakes(R)` and
`epoch_stakes(R + 1)`, when the vote account is included. Credits recorded in
`R` are `credits(R + 1) - credits(R)`, available at the start of `R + 1`.
The inflation commission delay does not define the effective epoch of the
block-revenue commission.

A missing epoch account is distinct from a missing entry. Absence of an account
outside the retention window says nothing about its vote-account membership.
These snapshots cover the admitted stake set, not every vote account. Absence
from this table alone does not establish a reward-calculation fallback. Missing
entries and closed-and-recreated vote accounts can prevent a valid credits
comparison. They do not represent zero commission or zero credits.

Credits are increments recorded during the indicated epoch, which may include
votes for earlier slots under Alpenglow. Per-epoch reward totals are out of scope;
these fields alone do not reconstruct actual delegator returns.

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
| `cumulative_credits` | `u64` | 72 |
| `inflation_rewards_commission_bps` | `u16` | 80 |
| `block_revenue_commission_bps` | `u16` | 82 |
| `alpenglow_rank` | `u16` | 84 |
| `_reserved` | `[u8; 10]` | 86 |
| `bls_pubkey_compressed` | `[u8; 48]` | 96 |
| `_reserved` | `[u8; 16]` | 144 |

Each admitted vote account appears once. The fields contain:

- `vote_pubkey`: the vote account address.
- `node_pubkey`: its validator identity at the snapshot.
- `delegated_stake`: the effective delegated stake in lamports selected at the
  snapshot. It is fixed even if subsequent reward payouts increase live stake.
- `cumulative_credits`: the last cumulative credits value in the snapshot's
  vote-state credits history, or zero when that history is empty.
- `inflation_rewards_commission_bps` and `block_revenue_commission_bps`: the raw
  vote-account v4 values. Every `u16` value is valid in this format, including
  values above 10,000. Publication **MUST NOT** clamp them. Reward calculation
  clamps to 10,000 when applying a rate. For pre-v4 vote states, publish the
  legacy inflation commission percentage multiplied by 100 and a block-revenue
  commission of 10,000, following [SIMD-0185].
- `alpenglow_rank`: the rank in the canonical Alpenglow BLS validator set for
  `E + 1`, including its BLS-key and identity deduplication. Rank zero has the
  highest eligible stake. Write `u16::MAX` for an entry outside that active set.
- `bls_pubkey_compressed`: the vote state's 48-byte compressed BLS public key,
  or 48 zero bytes if it has no BLS key.
- `_reserved`: 26 zero bytes. Reusing them requires a later protocol change
  that preserves existing readers' interpretation of version 1.

Collector addresses are excluded. Changes to which vote-account state supplies
reward collectors belong to a separate proposal; this account does not define
or change those rules.

The admission limit from [SIMD-0357] bounds each account to 2,000 entries.
The data length **MUST** be exactly `32 + num_entries * 160` bytes, at most
320,032 bytes (about 313 KiB). Eight retained accounts occupy at most
2,560,256 bytes (about 2.44 MiB), excluding account and storage metadata.

### Account Addresses and Owner

For the snapshot taken at the start of `E`:

```text
epoch_stakes(E) = PDA(program_id, [b"epoch_stakes", E.to_le_bytes()])
```

`E` is encoded as eight little-endian bytes. Each epoch has a distinct address;
its data is never rotated to another address or republished after closure.

The owner is a new native program, the Epoch Stakes program:

```text
EpochStakes11111111111111111111111111111111
```

The program rejects every instruction with `InvalidInstructionData`. Only the
runtime creates and closes its epoch stakes accounts. Transactions cannot
modify their data or owner, or debit their lamports through this program.

Neither the program ID nor the per-epoch PDAs are added to the reserved account
keys. Ordinary account-lock and program-account rules apply. Transactions can
fund a PDA without gaining control of it. Lamports sent to the program account
have no withdrawal path through this program.

### Publication and Funding

On the first block of each epoch `E` after activation, the runtime **MUST**:

1. Publish the snapshot defined under [Snapshot Timing](#snapshot-timing) at
   `epoch_stakes(E)`. Use that snapshot even if reward distribution has already
   changed live account balances by the time the publication is stored.
2. When `E > 0`, if `epoch_stakes(E - 1)` is missing and the snapshot used for
   consensus in `E` is available, publish it under `E - 1`. Use the cached
   snapshot, not live state. This supplies the current set on activation.
3. Preserve any existing lamports at each PDA being published. Set its balance
   to the greater of its existing balance and the rent-exempt minimum for the
   serialized data.
   Increase capitalization only by the shortfall supplied by the runtime.
4. Close expired accounts as specified below.

An over-funded PDA keeps its full balance; publication does not burn the excess
or reduce the balance to the rent-exempt minimum. Repeating publication for an
already-published epoch leaves both its data and balance unchanged. Transfers
to a retained account can increase its balance without changing the snapshot.

### Activation, Retention, and Closure

Activation **MUST** occur at an epoch boundary with [SIMD-0326] and [SIMD-0357]
active. The rank and entry-count bound depend on those proposals.

Let `A` be the first epoch boundary at which this feature publishes snapshots.
Publish `epoch_stakes(A)` and, when `A > 0` and the snapshot is available,
`epoch_stakes(A - 1)`. No earlier snapshot is backfilled, and no future snapshot
epoch is published.
After processing the boundary into epoch `C`, the retained range is:

```text
max(A.saturating_sub(1), C.saturating_sub(7)) <= snapshot_epoch <= C
```

This retains at most eight accounts: current and seven previous epochs. The
window fills over subsequent boundaries. If an epoch has no blocks, no snapshot
is taken for that epoch, so its account remains absent. Gaps do not extend the
lower bound, and a jump across multiple epochs still closes every previously
published account below it.

At each boundary, the runtime **MUST** clear the data and zero the lamport
balance of every expired epoch stakes account, reducing capitalization by its
full balance. The burn includes lamports transferred to the account by users.
Closure **MUST** affect only accounts owned by the Epoch Stakes program.

System-owned placeholders are left unchanged. These can exist before first
publication, in skipped epochs, or at expired addresses that users fund again
after closure. The owner check therefore remains necessary after the initial
eight epochs. Such placeholders do not count as published snapshots and are
never backfilled outside the retained range.

Consumers can authenticate a snapshot by checking the derived address, owner,
format version, header epoch, and data length. A funded PDA alone does not prove
publication. Older history requires an archive; neither missing snapshots nor
missing entries imply zero stake, credits, or commission.

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
under [SIMD-0180]; `node_pubkey` here is snapshot metadata.

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
snapshot. Consumers of commissions and credits use the timing rules above.

## Security Considerations

Account data is covered by the bank hash. Publication uses the existing
consensus snapshot and canonical rank map; it does not alter stake selection,
reward calculation, or snapshot loading.

The owner rejects all instructions, so receiving a transfer does not let a
transaction change snapshot contents. An account's lamport balance is its
funding balance, not its recorded validator stake. Consumers can inspect that
balance, but stake calculations use the serialized stake fields.

Publication preserves pre-funded balances and mints only a rent shortfall.
Closure burns the full expired balance. At 3,480 lamports per byte-year, a
two-year exemption threshold, and 128 bytes of account overhead, eight maximum
accounts need about 17.83 SOL if entirely funded by the runtime. Once the window
is full, an expired account offsets funding a new account of the same size.
Transfers can raise balances but cannot enlarge the account data.

## Future Work

Snapshot-manifest validation or replacement, per-epoch reward totals, and
changes to collector lookup remain separate proposals. This format omits
collector addresses and other internal cache data, so it does not by itself
replace every cached vote-account field.

Commission history that can activate before Alpenglow is also independent of
this proposal. The activation dependencies and admitted-set coverage here do
not provide that interim history.

## Backwards Compatibility

This introduces a new account format and native program under a feature gate.
It changes no existing account layout, sysvar, RPC method, or reward rule.
All validators must implement publication, funding, and closure before the
feature activates.

[SIMD-0180]: ./0180-vote-account-leader-schedule.md
[SIMD-0185]: ./0185-vote-account-v4.md
[SIMD-0249]: ./0249-delay-commission-updates.md
[SIMD-0326]: ./0326-alpenglow.md
[SIMD-0357]: ./0357-alpenglow_validator_admission_ticket.md
[SIMD-0558]: ./0558-leader-info-syscall.md
