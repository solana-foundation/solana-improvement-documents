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
commission, reward state, collectors, location, latest vote, root slot, and the
BLS key and signer index used for Alpenglow in `E + 1`.
Retain eight snapshots: the current epoch and seven previous epochs. Close
older accounts at epoch boundaries.

## Motivation

Historical epoch stakes currently live in validator-local caches and snapshot
manifests. Publishing per-validator stake weights, identities, and BLS keys and
signer indices as account data puts these consensus inputs under the bank hash.
Snapshot loading uses these accounts to rebuild the corresponding historical
stake and vote-state cache fields. This allows clients to retire duplicate
manifest data once their remaining legacy consumers no longer need it.

On-chain programs can inspect stake distributions, and read-layer clients can
map BLS signer indices in certificates to validators. Existing account reads and
subscriptions can serve this data without a new RPC method or Geyser interface.

The snapshot-loading transition below preserves existing reward calculations
and RPC methods. In-memory caches may remain. Removing fields from the snapshot
wire format requires a coordinated format change and is not required here.

## Dependencies

- [SIMD-0326] (Alpenglow), with the proposed [BLS Signer Indices] amendment,
  defines the signer indices recorded here.
- [SIMD-0357] (Validator Admission Ticket) defines the admitted stake set and
  the 2,000-entry limit.

Both features **MUST** be active before this proposal activates.

The location field uses the encoding in the proposed [SIMD-0674]. That feature
is not an activation prerequisite. Vote states without a location use the
absent value defined below.

## New Terminology

An **epoch stakes account** stores the snapshot taken at the start of its named
epoch. It is a runtime-managed account with a dedicated owner address and a
program-derived address (PDA) keyed by that epoch.

## Detailed Design

### Snapshot Timing

At the boundary into epoch `E`, take the snapshot after applying feature
activations, calculating stake activation and deactivation for `E`, and applying
the validator admission rules used to form the consensus stake set for `E + 1`.
Use vote-account state before sweeping pending delegator rewards or
distributing inflation rewards at this boundary
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

The key is the snapshot epoch. The stake set and BLS signer indices in
`epoch_stakes(E)` apply to consensus in `E + 1`.

For the same vote-account incarnation present in consecutive snapshots, let
`lamports(E)` be its `accumulated_lamports` in `epoch_stakes(E)`. When both
snapshots are within Alpenglow and the counter remains continuous and does not
saturate:

| Value | Epoch it describes |
|-------|--------------------|
| Stake set and BLS signer indices in `epoch_stakes(E)` | Consensus in `E + 1` |
| Inflation commission in `epoch_stakes(E)` | Rewards in `E + 1` |
| Block-revenue commission in `epoch_stakes(E)` | Block revenue in `E + 1` |
| `lamports(E) - lamports(E - 1)` | Reward lamports recorded during `E - 1` |
| `latest_vote` and `root_slot` | Vote-account state at the start of `E` |

For example, `epoch_stakes(100)` contains the snapshot taken at the start of
100. Its inflation and block-revenue commission rates apply in 101. Under the
conditions above, its accumulated lamports minus those in `epoch_stakes(99)`
give the reward lamports recorded during 99.

`latest_vote` and `root_slot` preserve the historical health indicators from
that same boundary snapshot. They are not live values and are not refreshed
during the epoch. A missing value is distinct from slot zero.

### Account Structure

Version 1 has an immutable binary layout and field meanings. Any change to
that layout or those meanings, including use of padding bytes for data,
**MUST** introduce a new version. All multi-byte integers are little-endian.
The header is 32 bytes, followed by `num_entries` entries of 264 bytes each.
Entries are sorted in ascending lexicographic order of `vote_pubkey` bytes.

| Header field | Type | Byte offset |
|--------------|------|-------------|
| `version` | `u32` | 0 |
| `num_entries` | `u32` | 4 |
| `epoch` | `u64` | 8 |
| `total_stake` | `u64` | 16 |
| `_padding` | `[u8; 8]` | 24 |

`version` is 1. `epoch` is the snapshot epoch used to derive the address.
`total_stake` is the sum of `delegated_stake` across all entries. Padding
bytes in both the header and entries **MUST** remain zero for version 1.

| Entry field | Type | Byte offset |
|-------------|------|-------------|
| `vote_pubkey` | `Pubkey` | 0 |
| `node_pubkey` | `Pubkey` | 32 |
| `delegated_stake` | `u64` | 64 |
| `accumulated_lamports` | `u64` | 72 |
| `inflation_rewards_commission_bps` | `u16` | 80 |
| `block_revenue_commission_bps` | `u16` | 82 |
| `bls_signer_index` | `u16` | 84 |
| `_padding` | `[u8; 10]` | 86 |
| `bls_pubkey_compressed` | `[u8; 48]` | 96 |
| `latest_vote` | `u64`, `u64::MAX` = none | 144 |
| `root_slot` | `u64`, `u64::MAX` = none | 152 |
| `pending_delegator_rewards` | `u64` | 160 |
| `last_reward_epoch` | `u64`, `u64::MAX` = none | 168 |
| `previous_accumulated_lamports` | `u64` | 176 |
| `inflation_rewards_collector` | `Pubkey` | 184 |
| `block_revenue_collector` | `Pubkey` | 216 |
| `location` | `[i32; 3]` | 248 |
| `_padding_tail` | `[u8; 4]` | 260 |

Each vote account selected at the boundary into `E` appears once. This snapshot can
include entries excluded from the BLS signer index map because they share a
validator identity or BLS key. Those entries retain their stake and commission
data and use `u16::MAX` for the signer index. The fields contain:

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
  vote-account commission rates in basis points effective in `E + 1`, using
  [Commission Schedule Selection](#commission-schedule-selection) when the
  vote-state format provides schedules. Every `u16` value is valid,
  including values above 10,000. Publication **MUST NOT** clamp them. Reward
  calculation clamps to 10,000 when applying a rate. For vote-state formats
  with only a legacy inflation commission percentage, publish that percentage
  multiplied by 100 and a block-revenue commission of 10,000, following
  [SIMD-0185].
- `bls_signer_index`: the index in the canonical Alpenglow BLS validator set for
  `E + 1`, constructed as specified in [BLS Signer Indices]. Index zero has the
  highest eligible stake. Write `u16::MAX` for an entry outside that signer set.
- `_padding`: 10 bytes that remain zero for version 1.
- `bls_pubkey_compressed`: the vote state's 48-byte compressed BLS public key,
  or 48 zero bytes if it has no BLS key.
- `latest_vote`: the last voted slot recorded in the snapshot's vote state,
  or `u64::MAX` when absent.
- `root_slot`: the root slot recorded in that vote state, or `u64::MAX` when absent.
- `pending_delegator_rewards`: the pending block-revenue rewards for delegators
  at the snapshot, before the boundary's sweep and distribution. Publish zero
  for vote-state formats without this field.
- `last_reward_epoch` and `previous_accumulated_lamports`: together with
  `accumulated_lamports`, the last recorded reward tuple
  `(epoch, cumulative, previous)`. For formats with `epoch_credits`, copy all
  three values from its last entry. Apply the same rule to the accumulated
  lamports history in newer formats. If the history is empty, publish
  `u64::MAX` for the epoch and zero for both counters. Preserve migration
  offsets and counter resets exactly. The recorded epoch can precede `E - 1`
  by more than eight epochs. Do not infer it or the previous counter from
  adjacent epoch stakes accounts.
- `inflation_rewards_collector` and `block_revenue_collector`: the collector
  addresses in the boundary vote state. For formats without these fields,
  use `vote_pubkey` and `node_pubkey`, respectively, following [SIMD-0232].
- `location`: the registered `(x, y, z)` WGS84 ECEF coordinates in meters,
  encoded as three signed little-endian integers in that order, as in
  [SIMD-0674]. Copy the boundary value without recomputing or rounding it.
  Publish `[0, 0, 0]` when the vote state has no registered location, including
  formats without the field. This proposal does not activate location
  registration or change validator eligibility.
- `_padding_tail`: four bytes that remain zero for version 1.

These collector fields are historical metadata, not a new payment rule.
Inflation collector lookup continues to use live vote-account state under
[SIMD-0232]. If [SIMD-0700] activates, block-revenue collector lookup also uses
live state. Otherwise its existing lookup rule remains in force. Publishing a
collector here does not make it the payment destination.

The admission limit from [SIMD-0357] bounds each account to 2,000 entries.
The data length **MUST** be exactly `32 + num_entries * 264` bytes, at most
528,032 bytes (about 516 KiB). Eight retained accounts occupy at most
4,224,256 bytes (about 4.03 MiB), excluding account and storage metadata.

### Commission Schedule Selection

For vote-state formats with commission schedules, including the proposed
VoteStateV5 schedules, each commission field **MUST** contain the rate effective
in `E + 1`. From the schedule in the boundary snapshot, select the entry with
the greatest effective epoch less than or equal to `E + 1`. Apply this lookup
independently to inflation and block-revenue commissions. Publish the selected
raw basis-point value without clamping. The full schedules remain in vote state
and are not included in the epoch stakes account.

For example, when publishing `epoch_stakes(100)`, a schedule with entries
effective in epochs 99 and 102 supplies the rate from entry 99. An entry
effective exactly in 101 would supply that rate instead. Vote-state formats
without schedules use the scalar rates and legacy conversions defined above.

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
active. The signer index and entry-count bound depend on those proposals.

Eight snapshots cover the [requested minimum of five epochs] for on-chain
delegation scoring and leave room for delayed reads. At the 2,000-entry limit,
this bounds live account data at about 4.03 MiB.

[requested minimum of five epochs]: https://github.com/solana-foundation/solana-improvement-documents/pull/676#issuecomment-5904637000

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

### Snapshot Loading and Migration

For a published snapshot epoch `S`, clients **MUST** rebuild the corresponding
cache fields from `epoch_stakes(S)` when loading a validator snapshot. Its
consensus epoch is `S + 1`. Load it from the same bank's restored account state,
including incremental-snapshot updates, rather than from a live RPC or another
fork. The usual snapshot hash and trust checks still apply.

Validate the PDA, owner, version, header epoch, exact length, entry-count bound,
zero padding, strictly increasing vote keys, and checked sum of delegated
stake. Rebuild the canonical BLS signer index map and verify the stored indices,
including excluded entries. The header's total includes every admitted entry,
so it need not equal the BLS-eligible total. A malformed published account or a
disagreement with manifest fields representing the same historical inputs
**MUST** fail snapshot loading.
Do not silently substitute manifest or current vote-account data.

Reconstruction **MUST** preserve the selected VAT membership, including entries
outside the BLS signer set. Restore stake and identity mappings, commissions,
BLS keys and indices, pending delegator rewards, collectors, location, and the
latest reward tuple using their definitions above. Boundary `S` supplies the
distribution vote-state view used when paying rewards for `S - 1`. Older
snapshots supply delayed commission and consensus inputs under their existing
rules. This does not add previous VAT payers to a distribution set.

The latest reward tuple is sufficient for the ordinary Alpenglow reward-points
calculation. It is not a replacement for the full history needed by Tower or
the Alpenglow migration-epoch calculation. Clients **MUST** retain that history
through any snapshot restore or pending reward recalculation that needs it.
They must also retain any other inputs absent from this format while a consumer
still requires them, such as legacy authorized-voter state or commission
schedule entries needed by a fallback lookup for another epoch. Do not
synthesize complete vote accounts with invented values for omitted fields.

Reward recalculation continues to load current stake accounts, stake history,
epoch-reward state, and the existing reward-epoch delegated-stake totals from
their established sources. The pre-reward `delegated_stake` field is not a
replacement for these inputs or for balances after partitioned payouts.

At activation `A`, historical cache entries derived before `A` still come from
the legacy snapshot representation. Keep that representation until every
required historical input has an on-chain source or is no longer needed. A
missing account is permitted only when publication was not required, such as
a pre-activation or skipped epoch, or an epoch outside retention. Such an
absence does not mean an empty validator set. Preserve any legacy input still
needed for that case, and reject a snapshot lacking a required input from
either source. Missing data for an epoch known to have published is an error.

Eight calendar epochs bound storage, not the number of reward-history entries.
An eight-epoch wait alone therefore does not authorize manifest removal.
Clients may retire duplicate historical fields only after verifying that all
consumers can use the accounts, including across activation, skipped epochs,
vote-account closure or recreation, and active partitioned rewards. A later
snapshot-format change can then remove the redundant wire fields without
changing this account layout or reward semantics.

## Alternatives Considered

### Sysvars

Making these accounts sysvars would require defining how a family of
per-epoch addresses interacts with sysvar access such as `sol_get_sysvar` and
write-lock demotion. A dedicated owner lets programs read them as ordinary
account data, without adding sysvar access or reserved-key rules.

### Publishing the Leader Schedule

[SIMD-0558] provides a separate interface for current and next leader lookup.
This proposal exposes its underlying stake data and BLS signer indices. The
interfaces share no account or ABI. Schedule derivation uses vote-account keys
under [SIMD-0180]. The `node_pubkey` here is snapshot metadata.

This format records the stake snapshot without prescribing how to derive a
leader schedule. Future leader-schedule changes, including those for multiple
concurrent proposers (MCP), can use the snapshot without changing its layout.
Changes to the meaning of an existing field would require a new version.

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
2,000-entry limit and eight-epoch window bound live data at about 4.03 MiB.
Existing RPC methods and validator configuration remain unchanged.

Programs can read the accounts directly. Indexers can use `accountSubscribe`,
`programSubscribe`, or Geyser account notifications and archive snapshots before
expiry. Consumers mapping certificate signer indices to validators use the
preceding epoch's snapshot. Consumers of commissions and accumulated rewards use
the timing rules above.

## Security Considerations

Account data is covered by the bank hash. Publication uses the existing
consensus snapshot and canonical signer index map. It does not alter stake
selection or reward calculation. Snapshot loading validates and uses the
published historical fields under the transition rules above.

Publication preserves pre-funded balances and mints only a rent shortfall.
Closure burns the full expired balance. Funding uses the current rent minimum.
At 5,080 lamports per byte and 128 bytes of account overhead, eight maximum
accounts need about 21.46 SOL if entirely funded by the runtime. Once the window
is full, an expired account offsets funding a new account of the same size.
Transfers can raise balances but cannot enlarge the account data.

## Future Work

Removing redundant snapshot wire fields, publishing per-epoch reward totals,
and changing collector lookup remain separate work. This format does not
serialize complete vote accounts or replace live stake balances.

## Backwards Compatibility

This introduces a new account format and owner address under a feature gate.
It changes no existing account layout, sysvar, RPC method, or reward rule.
All validators must implement publication, funding, closure, and the
snapshot-loading transition before the feature activates. Older snapshots
remain loadable through the legacy path when they predate activation.

[SIMD-0180]: ./0180-vote-account-leader-schedule.md
[SIMD-0185]: ./0185-vote-account-v4.md
[SIMD-0232]: ./0232-custom-commission-collector.md
[SIMD-0326]: ./0326-alpenglow.md
[BLS Signer Indices]: https://github.com/solana-foundation/solana-improvement-documents/pull/727
[SIMD-0357]: ./0357-alpenglow_validator_admission_ticket.md
[SIMD-0558]: ./0558-leader-info-syscall.md
[SIMD-0674]: https://github.com/solana-foundation/solana-improvement-documents/pull/674
[SIMD-0700]: https://github.com/solana-foundation/solana-improvement-documents/pull/700
