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

Store the epoch stakes in a dedicated on-chain account per epoch, keyed by
epoch number. The account includes the vote-account stake mapping, vote account
v4 reward fields, and Alpenglow validator rank data. All epochs from feature
activation forward remain accessible as persistent account state.

## Motivation

The epoch stake distribution (the mapping from vote account to delegated
lamports) is the fundamental input to consensus. Today it is derivable only
through RPC methods such as `getVoteAccounts`, or by reading validator-local
fields in a snapshot manifest. There is no on-chain representation. This
creates several problems:

**For on-chain programs.** Programs that want to reason about stake
distribution directly have no way to do so on-chain. Concrete use cases
that are currently impossible:

- Stake-weighted on-chain governance (verifying quorum thresholds, computing
  weighted votes).
- Stake-aware delegation strategies that do not depend on off-chain oracles.
- Client-side derivation of the leader schedule: given the epoch stakes and
  the deterministic shuffle algorithm, any consumer can compute the leader
  schedule without needing it served separately.
- Any on-chain protocol that needs to confirm "validator X currently has
  Y stake at epoch Z" as part of its logic.

**For snapshot integrity.** The epoch stakes are currently stored in the
snapshot manifest as validator-local data that is *not* covered by the
bank hash. A snapshot with corrupted or tampered epoch stakes will load
successfully and the node will not discover the corruption until later in
startup, when the data is used to compute the leader schedule or validate
votes. Moving the epoch stakes into an account places them inside the bank
hash that validators vote on, so any corruption is detected immediately at
snapshot load time, before any consensus decisions are made. This closes
a snapshot verification gap that exists today.

**For indexers and off-chain infrastructure.** There is no way to subscribe
to epoch stake changes. Consumers must poll RPC endpoints, introducing
latency and coupling to the validator's RPC surface. With the stakes stored
in on-chain accounts, Geyser plugins and websocket `accountSubscribe` calls
can deliver stake distribution updates at epoch boundaries via the same
machinery used for any other account.

**For RPC surface area reduction.** A broader architectural goal is to
shrink the validator's RPC surface, ideally removing RPC from the validator
entirely and moving it to a separate process or client-side library. Every
piece of runtime-managed data that lives only in RPC responses is a blocker
for that goal. Moving the epoch stakes into an account removes the blocker
for the stake distribution piece specifically. Deletion of
`getVoteAccounts` stake fields or equivalent endpoints is out of scope for
this SIMD, but this is a prerequisite for that future work.

The epoch stakes are already deterministically computed by every validator.
This proposal simply makes that data available as account state.

[SIMD-0326] adds one more reason to expose the same snapshot. Alpenglow's BLS
certificate format identifies validators by a stake-ordered rank, and
[SIMD-0357] limits its validator set. Publishing the BLS key and rank with each
epoch's stake entry lets the read layer serve the canonical validator set
without rebuilding the runtime's eligibility and deduplication rules.

## New Terminology

**Epoch stakes account:** A system-managed account (not a sysvar; see
[Alternatives Considered](#alternatives-considered)) that stores the
mapping from vote account to delegated stake (in lamports) for a single
epoch. One such account is written per epoch, addressed by a PDA derived
from the epoch number.

## Detailed Design

### Account Structure

A separate epoch stakes account is written for each epoch, addressed by a
PDA derived from the epoch number (see [Account Addresses](#account-addresses)).
Each account contains a self-describing binary layout with a sorted
mapping from vote account to delegated stake.

All multi-byte integers are little-endian. Header fields are ordered so
that each field falls on its natural alignment boundary without padding,
and the entries section starts on a 32-byte boundary for Pubkey alignment.

```text
┌──────────────────────────────────────────────────────────────────┐
│ Header (32 bytes)                                                │
│   version: u32          - format version (currently 1)           │
│   num_entries: u32      - vote accounts in table                 │
│   epoch: u64            - epoch these stakes are for             │
│   total_stake: u64      - sum of all delegated stake             │
│   _reserved: [u8; 8]    - reserved, must be zero                 │
├──────────────────────────────────────────────────────────────────┤
│ Entries (num_entries x 224 bytes)                                │
│   For each entry, sorted by vote_pubkey byte order:              │
│     vote_pubkey:                      Pubkey  (32 B, offset   0) │
│     node_pubkey:                      Pubkey  (32 B, offset  32) │
│     inflation_rewards_collector:      Pubkey  (32 B, offset  64) │
│     block_revenue_collector:          Pubkey  (32 B, offset  96) │
│     delegated_stake:                  u64     ( 8 B, offset 128) │
│     cumulative_credits:               u64     ( 8 B, offset 136) │
│     inflation_rewards_commission_bps: u16     ( 2 B, offset 144) │
│     block_revenue_commission_bps:     u16     ( 2 B, offset 146) │
│     alpenglow_rank:                   u16     ( 2 B, offset 148) │
│     _reserved:                        [u8;10] (10 B, offset 150) │
│     bls_pubkey_compressed:            [u8;48](48 B, offset 160) │
│     _reserved:                        [u8;16] (16 B, offset 208) │
└──────────────────────────────────────────────────────────────────┘
```

The `version` field is the first field in the header, enabling clients to
read the first four bytes to detect incompatible format changes and fail
gracefully rather than silently misparse account data. This proposal
defines version 1. Future SIMDs that alter the layout would increment the
version. A `u32` is used rather than `u16` because placing a 4-byte field
first allows the remaining header fields to be naturally aligned without
padding; the 2-byte cost is negligible relative to the per-account size.

The `total_stake` field is a convenience: consumers can verify it by
summing the individual entries. It is included so that programs that only
need the aggregate (e.g. for quorum thresholds) do not have to load and
sum the full table.

#### Per-Entry Fields

Each entry is 224 bytes. The 224-byte size keeps every entry on a 32-byte
boundary (since the entries section starts at offset 32 and 224 is a
multiple of 32), preserving zero-copy `Pubkey` reads across the whole
table. Entries are sorted by `vote_pubkey` to enable binary search.

The collector and commission fields mirror the vote account v4 state
introduced by [SIMD-0185] and consumed by [SIMD-0232]: v4 splits
validator income into two streams (inflation rewards and block revenue),
each with its own collector address and its own commission rate in basis
points. This schema carries both streams so that it remains a complete
replacement for the per-vote-account data the runtime reads at epoch
boundaries after those SIMDs are active.

- **`vote_pubkey`** - the vote account address. Primary key; entries are
  sorted by this field.
- **`node_pubkey`** - the validator identity address operating this vote
  account. Required for any consumer that needs to map a vote account back
  to its operator (e.g. snapshot manifest replacement, governance UIs that
  display operator identity).
- **`inflation_rewards_collector`** - the address that collects the
  inflation rewards commission for this vote account, as defined by
  SIMD-0185/SIMD-0232. For vote accounts whose state predates vote
  account v4, runtime implementations **MUST** populate this field with
  the same value as `vote_pubkey` (matching the SIMD-0185 migration
  default: inflation rewards commission was previously collected into
  the vote account itself).
- **`block_revenue_collector`** - the address that collects block fee
  revenue for this vote account, as defined by SIMD-0185/SIMD-0232. For
  vote accounts whose state predates vote account v4, runtime
  implementations **MUST** populate this field with the same value as
  `node_pubkey` (matching the SIMD-0185 migration default: block revenue
  was previously collected into the validator identity account). Both
  collector fields are included now rather than added in a future schema
  bump because any retroactive expansion would force a v2 format and
  break consumers reading v1.
- **`delegated_stake`** - total stake delegated to this vote account, in
  lamports. Sum of the individual stake delegations pointing at this vote
  account.
- **`cumulative_credits`** - total epoch credits earned by this vote
  account through the epoch this account represents (i.e. the latest
  cumulative credits value from the vote state's `epoch_credits` history
  as of this epoch). Reward distribution computes credits earned *in*
  epoch N by diffing this field between the epoch-N and epoch-(N-1)
  accounts.
- **`inflation_rewards_commission_bps`** - inflation rewards commission
  in basis points `[0, 10000]`, matching the vote account v4
  representation. For vote accounts whose state predates v4, runtime
  implementations **MUST** populate this field with the legacy `u8`
  percentage commission multiplied by 100 (the SIMD-0185 migration rule).
- **`block_revenue_commission_bps`** - block revenue commission in basis
  points `[0, 10000]`, matching the vote account v4 representation. For
  vote accounts whose state predates v4, runtime implementations **MUST**
  populate this field with `10000` (the SIMD-0185 migration default:
  100% of block revenue previously went to the validator).
- **`alpenglow_rank`** - the validator's canonical rank in the Alpenglow
  BLS validator set for this epoch. Rank zero is the highest-staked eligible
  validator. Implementations **MUST** write `u16::MAX` when the vote account
  is not in the active set or Alpenglow is not active. This preserves the
  runtime's BLS-key and identity deduplication result for read-layer clients.
- **`bls_pubkey_compressed`** - the 48-byte compressed BLS public key from
  vote account v4. Implementations **MUST** write 48 zero bytes when the vote
  account does not have a BLS key.
- **`_reserved`** - 26 zero bytes reserved for future use within this
  schema version. A future SIMD could repurpose these bytes for
  additional fields without bumping the version, provided it preserves
  read compatibility.

[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/pull/185
[SIMD-0232]: https://github.com/solana-foundation/solana-improvement-documents/pull/232
[SIMD-0326]: https://github.com/solana-foundation/solana-improvement-documents/pull/326
[SIMD-0357]: https://github.com/solana-foundation/solana-improvement-documents/pull/357

#### Schema Rationale

The per-entry fields cover the stake, reward, identity, and Alpenglow data that
read-layer consumers need. The runtime also caches derived maps such as epoch
authorized voters. Removing the internal `epoch_stakes` snapshot field remains
follow-up work and may require reconstructing those maps at snapshot load.

1. **Schema-as-ABI.** Once consumers (on-chain programs, light clients,
   indexers) are reading these accounts, the entry layout becomes an ABI.
   Retroactive expansion would force a v2 format and either break v1
   consumers or require dual-version support indefinitely.
2. **Cost is bounded.** Per-entry size grows from 40 bytes to 224 bytes,
   per-account size from ~80 KiB to ~438 KiB, and annual state growth from
   ~56 MiB/year to ~312 MiB/year. Preserving the history avoids rewriting
   old data and lets consumers query an epoch without maintaining an archive.
3. **Per-stake-delegation data is not included.** Agave no longer serializes
   stake delegations in `epoch_stakes`; they are recoverable from stake
   accounts. Reintroducing them here would add cost without restoring data the
   runtime still needs in that snapshot field.

### Size Analysis

With mainnet parameters (~2,000 vote accounts):

| Component | Calculation | Size |
|-----------|------------|------|
| Header | fixed | 32 bytes |
| Entries | 2,000 x 224 bytes | ~438 KiB |
| **Total per account** | | **~438 KiB** |

The epoch stakes account **MUST** contain at most 2,000 entries, matching
the validator admission cap enforced elsewhere in the protocol. This
bounds the maximum account size and gives consumers a firm upper limit
for buffer allocation.

At ~2 epochs per day, annual on-chain state growth from this proposal
is approximately 312 MiB/year. This is a deliberate tradeoff (see
[State Growth](#state-growth)). Preserving all history simplifies
consumer logic and avoids write amplification. The annual growth is
small relative to overall validator state and can be pruned in a
future SIMD if it becomes a concern.

### Account Addresses

Each epoch has its own account at a PDA keyed by epoch number:

```text
epoch_stakes(epoch) = PDA(program_id, [b"epoch_stakes", epoch.to_le_bytes()])
```

The `epoch` value is encoded as 8 little-endian bytes (matching the
on-wire `u64` representation of an epoch number).

Every epoch has its own stable, deterministic address. The runtime writes the
account data once and never copies it between addresses. A consumer that knows
an epoch number can derive the address and read the data for any epoch that has
been written since feature activation.

This scheme eliminates the write amplification of a rolling
`previous / current / next` layout (which would require rewriting the
same data under different addresses every epoch boundary). It also
naturally preserves historical stakes. Any program or indexer can walk
backward through epochs for retrospective analysis without needing an
off-chain archive.

Consumers subscribing to the current and upcoming epoch stakes can compute
both addresses from the current epoch number (available via the `Clock`
sysvar) and subscribe directly. Indexers that want to be notified of new
epochs can subscribe to program-owned accounts via `programSubscribe` or
equivalent Geyser filters.

### Owner Program

The accounts are owned by a new native program, the **Epoch Stakes
program**, with program ID:

```text
EpochStakes11111111111111111111111111111111
```

This is a name-based address with no known private key, following the
same convention as other native programs (`Stake11111111111111111111111111111111111111`,
`Vote111111111111111111111111111111111111111`, etc.). The program:

- Rejects all instructions, so transactions cannot change the account data,
  owner, or decrease its lamport balance.
- Serves only as the owner for the epoch stakes accounts.
- Is updated exclusively by the runtime at epoch boundaries.

### Runtime Behavior

#### Epoch Boundary Update

At each epoch boundary (when `parent.epoch() < new.epoch()`), the runtime:

1. Serializes the epoch stakes for `current_epoch + 1` (the vote account
   to stake mapping) into the binary format described above.
2. Creates the account at `epoch_stakes(current_epoch + 1)` with the
   serialized data and a rent-exempt lamport balance.

If the runtime finds that the current epoch's account is missing when
this logic runs (e.g. on the very first epoch boundary after feature
activation), it additionally writes the current epoch's account.

Each newly created account is funded with the rent-exempt minimum. The runtime
does not rewrite its data. There is no copying or rotation between addresses.
If someone pre-funds a future PDA, the runtime preserves those lamports when it
writes the account so pre-funding cannot block publication or destroy funds.

This integrates into the existing epoch-boundary processing in
`process_new_epoch()`, after vote account stake snapshots are taken and
`update_epoch_stakes()` has been called.

#### Feature Activation

The network feature **MUST NOT** activate before [SIMD-0326] and [SIMD-0357]. The
2,000-entry bound relies on the Alpenglow validator admission limit, and the
rank field relies on the canonical Alpenglow validator set.

On the first epoch boundary after feature activation, the runtime creates
the account for the current epoch and the account for the next epoch (if
the next epoch's stakes are already available). No historical accounts
are backfilled. Data for epochs preceding feature activation is not
available on-chain.

From that point forward, each epoch boundary writes at most one new
account (for `current_epoch + 1`), as described above.

Consumers **MUST** check that an account exists (via e.g.
`getAccountInfo`) before attempting to read it. Accounts for epochs
prior to feature activation, or for epochs further in the future than
the current leader schedule epoch, will not exist.

#### State Growth

Each epoch contributes ~438 KiB of new on-chain account state. With
approximately two epochs per day, annual growth is on the order of
312 MiB. This is a deliberate tradeoff: preserving all history simplifies
consumer logic, eliminates write amplification at epoch boundaries, and
enables retrospective analysis without off-chain archival.

Future SIMDs may introduce a pruning mechanism if long-term state growth
becomes a concern. Because each account is independently addressable by
epoch, pruning can be added without breaking the address scheme for the
remaining accounts.

#### Consistency

The epoch stakes written to these accounts are identical to the stakes
already used internally by the runtime for leader schedule computation
and for other consensus-related bookkeeping. The deterministic
computation is unchanged; this proposal only makes the existing data
visible as account state.

### RPC

No changes to existing RPC methods are required by this proposal. The
`getVoteAccounts` method continues to work as before.

However, once the epoch stakes are available as account data, the
stake-distribution portion of `getVoteAccounts` becomes redundant as
a validator-specific endpoint. Client libraries can implement the
equivalent query as a plain account read (via any accounts interface they
already use, including snapshot reads or account subscriptions). This
unblocks eventual deletion of the endpoint (or the stake-distribution
portion of it) in a future SIMD, which is a prerequisite for the broader
effort to remove RPC from the validator entirely. The deletion itself is
out of scope for this SIMD; this proposal provides only the on-chain data
source that makes deletion possible.

## Alternatives Considered

### Sysvar Accounts

The most natural approach would be to make this a sysvar account,
following the pattern of `SlotHashes`, `StakeHistory`, etc. However, the
sysvar infrastructure carries significant overhead:

- **Hardcoded cache:** The `SysvarCache` struct has a fixed field per
  sysvar. Adding a new sysvar requires modifications to ~15 files across
  the runtime, program-runtime, syscalls, SVM, and test infrastructure.
- **Per-bank caching:** Every bank creation populates the sysvar cache.
  For accounts that change only at epoch boundaries, this is unnecessary
  overhead.
- **Serialization constraints:** Sysvars traditionally use bincode
  serialization. The epoch stakes account benefits from a raw binary
  layout for zero-copy on-chain access.

A system-managed account owned by a dedicated native program provides
runtime-controlled data at well-known addresses without coupling to the sysvar
cache infrastructure. Programs read the account data directly, just as they
would any other account.

### Including the Leader Schedule in This SIMD

An earlier draft also published the full leader schedule. [SIMD-0558] now
proposes a narrow syscall for the current and next slot's leaders. That
interface answers the immediate on-chain lookup without exposing the full
schedule as account state or freezing its internal representation.

This proposal publishes the epoch-stakes input and the canonical Alpenglow
rank. Off-chain consumers can derive a schedule from the stake data. On-chain
programs that need the current or next leader can use SIMD-0558. The two
proposals do not share an account or ABI. A later change to how SIMD-0558
exposes that lookup would not require a change to this account format.

[SIMD-0180] defines the leader schedule in terms of vote-account keys rather than
identity keys. The primary key here is also `vote_pubkey`; `node_pubkey` is
metadata. Future leader-schedule representation changes therefore do not
require an epoch-stakes format change.

[SIMD-0558]: ./0558-leader-info-syscall.md
[SIMD-0180]: ./0180-vote-account-leader-schedule.md

### Fixed-Seed Rolling Accounts (Previous / Current / Next)

An earlier draft of this proposal used three fixed-seed PDAs
(`[b"previous_epoch_stakes"]`, `[b"current_epoch_stakes"]`,
`[b"next_epoch_stakes"]`) and rotated their contents at each epoch
boundary. This was rejected in favor of epoch-number-keyed seeds for
several reasons:

- **Write amplification.** Rotation requires rewriting the same data
  under different addresses every epoch, producing three writes per
  epoch boundary instead of one.
- **Bounded history.** A rolling layout only ever exposes three epochs.
  Any consumer that needs older history would need to maintain their
  own archive, defeating a core motivation of the proposal.
- **Ambiguity at epoch boundaries.** A `current_epoch_stakes` account
  has an implicit epoch binding that changes on every epoch boundary,
  creating a race between the runtime write and any consumer reading
  the account. With epoch-keyed addresses, the account for a given
  epoch is written once and is unambiguous.

Reviewers on the SIMD discussion (trent-nelson, brooksprumo, joncinque)
converged on epoch-keyed addressing as the cleaner design.

### Geyser Plugin Interface

The Geyser plugin interface could be extended to emit epoch stakes data
directly at epoch boundaries without storing it in an account. However,
Geyser is a push-only interface: plugins receive notifications but
cannot query the validator for data on demand. A consumer that starts
mid-epoch, reconnects after a disconnect, or simply needs the current
stakes at an arbitrary point in time would have no way to retrieve them
without maintaining its own state from the stream origin.

Storing the stakes in an account solves this naturally. Any consumer can
read the account at any time via the existing accounts infrastructure
(snapshots, `getAccountInfo`, Geyser account notifications). This also
avoids adding request-response semantics to Geyser while RPC functionality is
being moved out of the validator.

### Mapping in a Single Combined Account

Storing multiple epochs' worth of stakes in one combined account would
reduce account count but increase per-account size and require
rewriting the account every epoch. Keeping one account per epoch eliminates
data rewrites and allows independent addressability.

## Impact

**Validator operators.** Validators will create one new account per
epoch boundary (~438 KiB) after the feature is activated, contributing
approximately 312 MiB per year of on-chain state growth. The account data is
written once at each epoch boundary, adding negligible overhead to epoch
processing. No configuration changes are required.

**RPC providers.** No immediate changes. Existing `getVoteAccounts` and
related endpoints continue to function. Over time, client libraries are
expected to shift to reading the on-chain accounts directly, which
opens the path to deprecating and eventually removing the
stake-distribution portion of `getVoteAccounts`, a prerequisite for
the broader effort to remove RPC from the validator itself.

**Indexers and Geyser plugin operators.** One of the primary
beneficiaries. Indexers can subscribe to the epoch stakes program via
`programSubscribe` (or the Geyser equivalent) to receive stake
distribution updates at epoch boundaries, replacing RPC polling.
Consumers that want only a specific epoch can subscribe to the
corresponding epoch-keyed PDA directly.

**On-chain program developers.** Programs can read the epoch stakes
account for any epoch since feature activation by deriving the PDA
from the epoch number. The binary format supports zero-copy access.
Concrete use cases include stake-weighted governance, quorum
verification, client-side leader schedule derivation, and stake-aware
delegation strategies.

**Core contributors.** This proposal introduces a new pattern for
runtime-managed accounts (non-sysvar, owned by a native program that
rejects all instructions). This pattern may be reused for other large,
infrequently-updated state that does not warrant the overhead of the
sysvar cache.

## Security Considerations

### Account Size and Growth

Each account is bounded at ~438 KiB by the 2,000-validator admission cap.
New accounts are created at each epoch boundary, yielding approximately
312 MiB of annual state growth. This is a deliberate tradeoff discussed
under [State Growth](#state-growth); pruning may be introduced in a
follow-up SIMD if warranted.

### Capitalization Impact

Each epoch boundary adds enough lamports to make one new account rent exempt.
Without pre-funding, this is ~3.12 SOL per epoch at current rent parameters for
a ~438 KiB account, or approximately 2,280 SOL per year. These lamports are not
burned; they remain in the epoch stakes program's accounts indefinitely. If a
future pruning SIMD is adopted, freed lamports could be returned to the
treasury or burned.

### Read-Only Guarantees

The accounts are owned by a native program that rejects all instructions. A
transaction cannot change their data or owner and cannot decrease their
lamport balance. It can credit lamports to an account included as writable;
this does not change the serialized epoch stakes.

Because each epoch's account is addressed by a deterministic PDA, the
set of currently-reserved addresses is unbounded over time and cannot
be enumerated statically. The program ID itself is added to the
reserved account keys list to prevent any transaction from acquiring a write
lock on the program. The per-epoch PDAs are not reserved. This is sufficient
to protect their contents because:

1. No valid transaction can alter the data, owner, or debit lamports from an
   account whose owner program rejects every instruction.
2. The runtime's writes happen outside of transaction processing and
   are therefore unaffected by the account-lock scheduler.
3. Any validator that diverged from the deterministic computation would
   produce a different bank hash and fail consensus.

Programs reading the account must ignore its lamport balance. The serialized
stake data has the same consensus integrity regardless of later credits.

### Determinism

The epoch stakes are deterministically computed by the runtime from the
stake state at the epoch boundary. All validators produce identical
account contents for the same epoch, ensuring consensus on account
state.

## Future Work

The following items are explicitly out of scope for this SIMD but are
naturally enabled by it or complement it:

- **RPC method deletion.** The stake-distribution portion of
  `getVoteAccounts` (and related endpoints) can eventually be removed
  once clients have migrated to reading the on-chain account. This is
  a prerequisite for the broader effort to shrink the validator's RPC
  surface area and eventually remove RPC from the validator entirely.
  This SIMD does not delete anything. It provides the on-chain data
  source that makes a future deletion SIMD possible.
- **Snapshot manifest slimming.** Agave has already stopped serializing stake
  delegations in `epoch_stakes`. A follow-up can remove the remaining
  per-vote-account snapshot data after the loader can rebuild its derived maps
  from account state. This SIMD supplies the bank-hashed source data but does
  not change snapshot loading.
- **State pruning.** A maximum retention window (e.g. the most recent
  N epochs) if long-term growth becomes a concern.
- **Leader contact info via Geyser (planned follow-up, not a SIMD).**
  Streaming leader TPU contact info (gossip-level endpoints that can
  change within an epoch) will be delivered as a direct extension to
  the Geyser plugin interface rather than as a SIMD, since Geyser is
  outside the SIMD process. That work is independent of this proposal
  because the data has different properties (mutable, high-frequency,
  non-consensus) and a different appropriate delivery mechanism. It
  is the last piece needed for transaction senders to operate entirely
  RPC-free; this SIMD is one of the prerequisites.

## Backwards Compatibility

This proposal introduces a new account type and a new native program.
It does not modify any existing accounts, programs, sysvars, or RPC
methods. There are no backwards compatibility concerns.

Validators that have not activated the feature will not create or
update these accounts. Once the feature is activated network-wide, all
validators will maintain consistent account state.
