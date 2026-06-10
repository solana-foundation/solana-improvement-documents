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

Store the epoch stakes (the mapping from vote account to delegated stake) in
a dedicated on-chain account per epoch, keyed by epoch number. All epochs
from feature activation forward remain accessible as persistent account
state, enabling on-chain programs, indexers, and snapshot-consuming nodes
to read the stake distribution directly without relying on RPC.

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

## New Terminology

**Epoch stakes account:** A system-managed account (not a sysvar — see
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

```
┌───────────────────────────────────────────────────────────┐
│ Header (32 bytes)                                         │
│   version: u32          — format version (currently 1)    │
│   num_entries: u32      — vote accounts in table          │
│   epoch: u64            — epoch these stakes are for      │
│   total_stake: u64      — sum of all delegated stake      │
│   _reserved: [u8; 8]    — reserved, must be zero          │
├───────────────────────────────────────────────────────────┤
│ Entries (num_entries × 128 bytes)                         │
│   For each entry, sorted by vote_pubkey byte order:       │
│     vote_pubkey:           Pubkey  (32 bytes, offset   0) │
│     node_pubkey:           Pubkey  (32 bytes, offset  32) │
│     commission_collector:  Pubkey  (32 bytes, offset  64) │
│     delegated_stake:       u64     ( 8 bytes, offset  96) │
│     cumulative_credits:    u64     ( 8 bytes, offset 104) │
│     commission:            u8      ( 1 byte,  offset 112) │
│     _reserved:             [u8;15] (15 bytes, offset 113) │
└───────────────────────────────────────────────────────────┘
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

Each entry is 128 bytes. The 128-byte size keeps every entry on a 32-byte
boundary (since the entries section starts at offset 32 and 128 is a
multiple of 32), preserving zero-copy `Pubkey` reads across the whole
table. Entries are sorted by `vote_pubkey` to enable binary search.

- **`vote_pubkey`** — the vote account address. Primary key; entries are
  sorted by this field.
- **`node_pubkey`** — the validator identity address operating this vote
  account. Required for any consumer that needs to map a vote account back
  to its operator (e.g. snapshot manifest replacement, governance UIs that
  display operator identity).
- **`commission_collector`** — the address that collects validator
  commission for this vote account. This field is reserved for the custom
  commission collector account introduced by SIMD-0232. Until that SIMD
  is active, runtime implementations **MUST** populate this field with
  the same value as `node_pubkey`. After SIMD-0232 activation, runtime
  implementations **MUST** populate this field with the per-vote-account
  collector address as defined by that SIMD. The field is included now
  rather than added in a future schema bump because any retroactive
  expansion would force a v2 format and break consumers reading v1.
- **`delegated_stake`** — total stake delegated to this vote account, in
  lamports. Sum of the individual stake delegations pointing at this vote
  account.
- **`cumulative_credits`** — total epoch credits earned by this vote
  account through the epoch this account represents (i.e. the latest
  cumulative credits value from the vote state's `epoch_credits` history
  as of this epoch). Reward distribution computes credits earned *in*
  epoch N by diffing this field between the epoch-N and epoch-(N-1)
  accounts.
- **`commission`** — validator commission as a percentage in `[0, 100]`
  (matching the existing on-chain vote state representation).
- **`_reserved`** — 15 zero bytes reserved for future use within this
  schema version. A future SIMD could repurpose these bytes for
  additional fields without bumping the version, provided it preserves
  read compatibility.

#### Schema Rationale

The per-entry fields together cover the full per-vote-account data set
that the runtime currently reads from the bank's `epoch_stakes` field
during reward distribution and other epoch-boundary processing. Defining
the full per-vote-account schema in this SIMD (rather than incrementally
expanding it across multiple SIMDs) is a deliberate choice:

1. **Schema-as-ABI.** Once consumers (on-chain programs, light clients,
   indexers) are reading these accounts, the entry layout becomes an ABI.
   Retroactive expansion would force a v2 format and either break v1
   consumers or require dual-version support indefinitely.
2. **Cost is small.** Per-entry size grows from 40 bytes to 128 bytes,
   per-account size from ~80 KB to ~250 KB, and annual state growth from
   ~59 MB/year to ~178 MB/year. This is still well within the deliberate
   tradeoff envelope for "preserve all history simplifies consumer logic
   and avoids write amplification."
3. **Per-stake-delegation data is *not* included.** The complete
   replacement of the bank's `epoch_stakes` field also requires the
   per-stake-delegation list (`stake_account_pubkey`, `vote_account_pubkey`,
   `delegation_amount`, `activation_epoch`, `deactivation_epoch`,
   `credits_observed`). Including that on-chain historically would impose
   a perf and storage cost the use cases do not justify; the per-stake
   data for the current epoch is recoverable directly from the on-chain
   stake accounts at snapshot load. See [Future Work](#future-work) for
   how full snapshot-manifest removal proceeds without putting that data
   on-chain.

### Size Analysis

With mainnet parameters (~2,000 vote accounts):

| Component | Calculation | Size |
|-----------|------------|------|
| Header | fixed | 32 bytes |
| Entries | 2,000 × 128 bytes | ~250 KB |
| **Total per account** | | **~250 KB** |

The epoch stakes account **MUST** contain at most 2,000 entries, matching
the validator admission cap enforced elsewhere in the protocol. This
bounds the maximum account size and gives consumers a firm upper limit
for buffer allocation.

At ~2 epochs per day, annual on-chain state growth from this proposal
is approximately 178 MB/year. This is a deliberate tradeoff (see
[State Growth](#state-growth)) — preserving all history simplifies
consumer logic and avoids write amplification. The annual growth is
small relative to overall validator state and can be pruned in a
future SIMD if it becomes a concern.

### Account Addresses

Each epoch has its own account at a PDA keyed by epoch number:

```
epoch_stakes(epoch) = PDA(program_id, [b"epoch_stakes", epoch.to_le_bytes()])
```

The `epoch` value is encoded as 8 little-endian bytes (matching the
on-wire `u64` representation of an epoch number).

Every epoch has its own stable, deterministic address. Accounts are never
modified in place once written, and never copied between addresses. A
consumer that knows an epoch number can derive the address and read the
data for any epoch that has been written since feature activation.

This scheme eliminates the write amplification of a rolling
`previous / current / next` layout (which would require rewriting the
same data under different addresses every epoch boundary). It also
naturally preserves historical stakes — any program or indexer can walk
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

```
EpochStakes11111111111111111111111111111111
```

This is a name-based address with no known private key, following the
same convention as other native programs (`Stake11111111111111111111111111111111111111`,
`Vote111111111111111111111111111111111111111`, etc.). The program:

- Rejects all instructions (the accounts are read-only from the
  perspective of transactions).
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

Each newly created account is funded with the rent-exempt minimum and is
never modified again. There is no copying or rotation between addresses.

This integrates into the existing epoch-boundary processing in
`process_new_epoch()`, after vote account stake snapshots are taken and
`update_epoch_stakes()` has been called.

#### Feature Activation

On the first epoch boundary after feature activation, the runtime creates
the account for the current epoch and the account for the next epoch (if
the next epoch's stakes are already available). No historical accounts
are backfilled — data for epochs preceding feature activation is not
available on-chain.

From that point forward, each epoch boundary writes at most one new
account (for `current_epoch + 1`), as described above.

Consumers **MUST** check that an account exists (via e.g.
`getAccountInfo`) before attempting to read it. Accounts for epochs
prior to feature activation, or for epochs further in the future than
the current leader schedule epoch, will not exist.

#### State Growth

Each epoch contributes ~250 KB of new on-chain account state. With
approximately two epochs per day, annual growth is on the order of
178 MB. This is a deliberate tradeoff: preserving all history simplifies
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

A system-managed account owned by a dedicated native program achieves the
same goals (runtime-controlled, read-only, well-known addresses) without
coupling to the sysvar cache infrastructure. Programs read the account
data directly, just as they would any other account.

### Including the Leader Schedule in This SIMD

An earlier draft of this proposal included the full leader schedule
alongside the epoch stakes as a second account type per epoch. That
scope was deferred because:

- The leader schedule is deterministically derivable from the epoch
  stakes (via the ChaCha20 stake-weighted shuffle), so no use case is
  *blocked* purely by the leader schedule not being on-chain — an
  off-chain or client-side derivation path always exists.
- The only use cases that are strictly unlocked by having the leader
  schedule itself in account state are on-chain programs that need the
  schedule inside a transaction and cannot afford to run the shuffle
  themselves (the shuffle is ~108,000 stake-weighted samples, too
  expensive for a program's CU budget).
- Demand for on-chain leader-schedule access is speculative today. A
  follow-up SIMD can introduce the schedule account type if concrete
  use cases materialize (see [Future Work](#future-work)).

Keeping this SIMD focused on epoch stakes reduces scope, reduces total
on-chain state growth (~178 MB/year vs. ~488 MB/year for the combined
proposal), and matches the set of use cases that have concrete demand
today.

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
avoids adding request-response semantics to Geyser, which would conflict
with the ongoing effort to decouple RPC-adjacent functionality from the
validator.

### Mapping in a Single Combined Account

Storing multiple epochs' worth of stakes in one combined account would
reduce account count but increase per-account size and require
rewriting the account every epoch. Keeping one account per epoch
eliminates rewrites and allows independent addressability, matching the
design goal of "write once, never modify."

## Impact

**Validator operators.** Validators will create one new account per
epoch boundary (~250 KB) after the feature is activated, contributing
approximately 178 MB per year of on-chain state growth. The account is
written once at each epoch boundary and never modified thereafter,
adding negligible overhead to epoch processing. No configuration
changes are required.

**RPC providers.** No immediate changes. Existing `getVoteAccounts` and
related endpoints continue to function. Over time, client libraries are
expected to shift to reading the on-chain accounts directly, which
opens the path to deprecating and eventually removing the
stake-distribution portion of `getVoteAccounts` — a prerequisite for
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

Each account is bounded at ~250 KB by the 2,000-validator admission cap.
New accounts are created at each epoch boundary, yielding approximately
178 MB of annual state growth. This is a deliberate tradeoff discussed
under [State Growth](#state-growth); pruning may be introduced in a
follow-up SIMD if warranted.

### Capitalization Impact

Each epoch boundary mints the rent-exempt minimum for one new account
(~1.75 SOL per epoch at current rent parameters for a ~250 KB account).
Over a year this amounts to approximately 1,270 SOL transferred into
rent-exempt account balances. These lamports are not burned — they
remain in the epoch stakes program's accounts indefinitely. If a future
pruning SIMD is adopted, freed lamports could be returned to the
treasury or burned.

### Read-Only Guarantees

The accounts are owned by a native program that rejects all
instructions. There is no code path by which a transaction can modify
them — the only writer is the runtime itself, acting at epoch
boundaries.

Because each epoch's account is addressed by a deterministic PDA, the
set of currently-reserved addresses is unbounded over time and cannot
be enumerated statically. The program ID itself is added to the
reserved account keys list to prevent any transaction from acquiring a
write lock on the program, and all accounts owned by this program
inherit the rejection behavior through the owner check. This is
sufficient because:

1. No valid transaction can mutate an account whose owner program
   rejects every instruction.
2. The runtime's writes happen outside of transaction processing and
   are therefore unaffected by the account-lock scheduler.
3. Any validator that diverged from the deterministic computation would
   produce a different bank hash and fail consensus.

Combined, these provide the same integrity guarantee as sysvar
accounts.

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
  This SIMD does not delete anything — it provides the on-chain data
  source that makes a future deletion SIMD possible.
- **Snapshot manifest slimming (multi-step).** Full removal of the
  bank's `epoch_stakes` field from the snapshot manifest proceeds in
  two steps. Step one (this SIMD) moves the per-vote-account portion
  on-chain via the schema defined above, allowing the manifest to drop
  those fields. Step two — out of scope here and likely an
  implementation-only change rather than a SIMD — replaces the
  per-stake-delegation portion by recovering it at snapshot load from
  the on-chain stake accounts (which already carry that data
  authoritatively). The per-stake-delegation list is intentionally
  *not* added to the on-chain epoch stakes account because the
  perf/storage cost of doing so historically is not justified by the
  use cases. The verifiability benefit described in
  [Motivation](#motivation) is immediate for the per-vote-account data
  this SIMD covers and does not depend on either follow-up.
- **State pruning.** A maximum retention window (e.g. the most recent
  N epochs) if long-term growth becomes a concern.
- **On-chain leader schedule (planned sidecar SIMD).** A follow-up SIMD
  will propose adding the full per-epoch leader schedule as a second
  account type owned by the same or a sibling native program. That
  SIMD is scoped independently because its value depends on concrete
  on-chain use cases (e.g. permissionless skip-rate cranking
  programs). Leaving the leader schedule out of this SIMD keeps the
  state growth bounded at ~178 MB/year and focuses the initial feature
  on the use cases with existing demand.
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
