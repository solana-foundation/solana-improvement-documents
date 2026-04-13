---
simd: '0511'
title: On-Chain Leader Schedule and Epoch Stakes
authors:
  - sam0x17
category: Standard
type: Core
status: Review
created: 2026-03-23
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Store the leader schedule and epoch stakes (vote account to delegated stake
mapping) for each epoch in dedicated on-chain accounts, keyed by epoch number.
This enables subscription-based access for indexers, permissionless on-chain
skip rate computation, and a general-purpose on-chain view of the stake
distribution. All epochs from feature activation forward remain accessible
as persistent account state.

## Motivation

The Solana leader schedule is currently only accessible through RPC methods
(`getLeaderSchedule`, `getSlotLeaders`). This creates several problems:

**For indexers and off-chain infrastructure:** There is no way to subscribe to
leader schedule changes. Consumers must poll RPC endpoints, introducing latency
and unnecessary load. With the schedule stored in on-chain accounts, Geyser
plugins and websocket `accountSubscribe` calls can deliver schedule updates in
real time at epoch boundaries.

**For skip rate computation:** Validator skip rates require correlating the slot
history with the leader schedule. Today this requires off-chain RPC polling.
With the schedule available on-chain (including the previous epoch), a
permissionless crank program can compute skip rates entirely on-chain by
combining the leader schedule account with the slot history sysvar. This
enables fully on-chain stake delegation strategies based on validator
performance, without relying on off-chain oracles.

**For stake distribution visibility:** The epoch stakes — the mapping from vote
account to delegated lamports — are a fundamental input to consensus but are
not currently accessible on-chain. Making this mapping available enables
programs to reason about the stake distribution directly: verifying quorum
thresholds, building stake-weighted governance, or computing the leader
schedule client-side from its inputs.

**For off-chain analytics:** Analytics pipelines can subscribe to the leader
schedule and epoch stakes accounts alongside the slot history sysvar for a
fully reactive approach to performance monitoring, without polling RPC
endpoints.

The leader schedule and epoch stakes are already deterministically computed by
every validator. This proposal simply makes that data available as account
state.

## New Terminology

**Leader schedule account:** A system-managed account (not a sysvar — see
[Alternatives Considered](#alternatives-considered)) that stores the unique
leader identity/vote-account pairs and the slot-to-leader mapping for a single
epoch. One such account is written per epoch, addressed by a PDA derived from
the epoch number.

**Leader span:** A contiguous group of slots assigned to a single leader
(currently 4 slots, i.e. `NUM_CONSECUTIVE_LEADER_SLOTS`). The schedule is
indexed by leader span rather than by individual slot.

**Epoch stakes account:** A system-managed account that stores the mapping
from vote account to delegated stake (in lamports) for a single epoch. One
such account is written per epoch, addressed by a PDA derived from the
epoch number.

## Detailed Design

### Account Structure

A separate leader schedule account is written for each epoch, addressed by a
PDA derived from the epoch number (see [Account Addresses](#account-addresses)).
Each account contains a self-describing binary layout with the identity table
(including vote addresses) and the schedule index array.

All multi-byte integers are little-endian. Header fields are ordered so that
each field falls on its natural alignment boundary without padding (version
u32 first for early detection, then u32, u64, u16, u16).

```
┌───────────────────────────────────────────────────────┐
│ Header (64 bytes)                                     │
│   version: u32              — format version (1)      │
│   num_leader_spans: u32     — leader spans in schedule│
│   epoch: u64                — epoch this schedule is  │
│   num_leaders: u16          — unique entries in table  │
│   slots_per_span: u16       — slots per leader span   │
│   _reserved: [u8; 12]       — reserved, must be zero  │
│   epoch_stakes_hash: [u8; 32] — hash of input stakes  │
├───────────────────────────────────────────────────────┤
│ Identity Table (num_leaders × 64 bytes)               │
│   entries: [(Pubkey, Pubkey); num_leaders]            │
│   — (validator identity, vote account) pairs,         │
│     sorted by identity key byte order                 │
├───────────────────────────────────────────────────────┤
│ Schedule (num_leader_spans × 2 bytes)                 │
│   leader_indices: [u16; num_leader_spans]             │
│   — index into Identity Table per leader span         │
└───────────────────────────────────────────────────────┘
```

The header is 64 bytes so that all sections start on 32-byte boundaries,
enabling zero-copy access to Pubkey entries without unaligned reads. The 12
reserved bytes between `slots_per_span` and `epoch_stakes_hash` must be zero
and are available for future use.

The `epoch_stakes_hash` field contains the SHA-256 hash of the epoch stakes
that were input to the leader schedule computation (the ChaCha20
stake-weighted shuffle). This allows consumers to verify that a given schedule
was derived from the expected stake distribution, without requiring the full
epoch stakes to be available on-chain. The hash is computed over the
stake-sorted `(vote_account, stake)` pairs in the same order used by the
schedule computation.

The `version` field is the first field in the header, enabling clients to read
the first four bytes to detect incompatible format changes and fail gracefully
rather than silently misparse account data. This proposal defines version 1.
Future SIMDs that alter the layout (e.g. wider indices, additional fields)
would increment the version. A `u32` is used rather than `u16` because placing
a 4-byte field first allows the remaining header fields to be naturally aligned
without padding, and the 2-byte cost is negligible relative to the ~344 KB
account size.

A **leader span** refers to a group of consecutive slots assigned to a single
leader. The `slots_per_span` field records how many slots comprise each span
(currently 4, i.e. `NUM_CONSECUTIVE_LEADER_SLOTS`). Consumers **must** read
this field from the header rather than hardcoding the divisor. This ensures
the format remains valid if the number of consecutive leader slots changes in
a future consensus update (e.g. under Alpenglow).

Each entry in the Identity Table is a 64-byte pair of `(identity, vote_account)`.
A single validator identity may appear in multiple entries if it operates
multiple vote accounts, since leader slots are assigned per vote account stake.
This enables per-vote-account skip rate computation, which is required for
accurate validator performance analysis and stake delegation decisions.

### Size Analysis

With mainnet parameters (432,000 slots/epoch, ~2,000 active validators):

| Component | Calculation | Size |
|-----------|------------|------|
| Header | fixed | 64 bytes |
| Identity Table | 2,000 × 64 bytes | 128 KB |
| Schedule | 108,000 × 2 bytes | 216 KB |
| **Total per account** | | **~344 KB** |

The identity table uses 64 bytes per entry (32-byte identity + 32-byte vote
account). Note that `num_leaders` may exceed the number of unique validator
identities if any identity operates multiple vote accounts.

The leader schedule program **MUST** reject any epoch whose identity table
would exceed 2,000 entries, matching the validator admission cap enforced
elsewhere in the protocol. This bounds the maximum leader schedule account
size to the values shown above and gives consumers a firm upper limit for
buffer allocation.

With `u16` indices, the identity table format supports up to 65,535 unique
entries, leaving substantial headroom above the 2,000 cap. If a future SIMD
raises the validator cap beyond this limit, a new format version with wider
indices could be introduced.

**Note on index width:** This proposal uses `u16` indices (2 bytes) rather than
`u32` (4 bytes), saving 216 KB per account at current mainnet parameters. The
`u16` limit of 65,535 entries provides substantial headroom. If the network
were to exceed this limit, a future SIMD could introduce a new version with
wider indices.

### Epoch Stakes Account Structure

A separate epoch stakes account is written for each epoch, also addressed by
a PDA derived from the epoch number. Each account contains a sorted mapping
from vote account to delegated stake.

```
┌───────────────────────────────────────────────────────┐
│ Header (32 bytes)                                     │
│   version: u32          — format version (currently 1)│
│   num_entries: u32      — vote accounts in table      │
│   epoch: u64            — epoch these stakes are for  │
│   total_stake: u64      — sum of all delegated stake  │
│   _reserved: [u8; 8]    — reserved, must be zero      │
├───────────────────────────────────────────────────────┤
│ Entries (num_entries × 40 bytes)                      │
│   entries: [(Pubkey, u64); num_entries]               │
│   — (vote account, delegated stake in lamports),      │
│     sorted by vote account pubkey byte order          │
└───────────────────────────────────────────────────────┘
```

The header is 32 bytes so that the entries section starts on a 32-byte
boundary for Pubkey alignment. The `total_stake` field is a convenience —
consumers can verify it by summing the individual entries.

Each entry is 40 bytes: a 32-byte vote account pubkey followed by an 8-byte
little-endian stake value. Entries are sorted by vote account pubkey to
enable binary search.

#### Epoch Stakes Size Analysis

With mainnet parameters (~2,000 vote accounts):

| Component | Calculation | Size |
|-----------|------------|------|
| Header | fixed | 32 bytes |
| Entries | 2,000 × 40 bytes | 80 KB |
| **Total per account** | | **~80 KB** |

The epoch stakes account **MUST** contain at most 2,000 entries, matching the
validator admission cap. Combined with the leader schedule account, each epoch
contributes approximately 424 KB of new on-chain state.

The `epoch_stakes_hash` in the leader schedule header is the SHA-256 hash
of the epoch stakes account data for the corresponding epoch (excluding the
header). This allows consumers to cross-reference the two account types and
verify that the leader schedule was derived from the published stakes.

### Account Addresses

Accounts live at Program Derived Addresses (PDAs) keyed by epoch number:

```
schedule(epoch)     = PDA(program_id, [b"schedule",     epoch.to_le_bytes()])
epoch_stakes(epoch) = PDA(program_id, [b"epoch_stakes", epoch.to_le_bytes()])
```

The `epoch` value is encoded as 8 little-endian bytes (matching the on-wire
`u64` representation of an epoch number).

Every epoch has its own stable, deterministic address. Accounts are never
modified in place once written, and never copied between addresses. A consumer
that knows an epoch number can derive the address and read the data for any
epoch that has been written since feature activation.

This scheme eliminates the write amplification of a rolling `previous /
current / next` layout (which would require rewriting the same data under
different addresses every epoch boundary). It also naturally preserves
historical schedules — any program or indexer can walk backward through epochs
for retrospective analysis without needing an off-chain archive.

Consumers subscribing to the current and upcoming epoch schedules can compute
both addresses from the current epoch number (available via the `Clock`
sysvar) and subscribe directly. Indexers that want to be notified of new
epochs can subscribe to program-owned accounts via `programSubscribe` or
equivalent Geyser filters.

### Owner Program

All six accounts are owned by a new native program, the **Leader Schedule
program**, with program ID `TBD` (to be derived/assigned before this SIMD is
finalized). This program:

- Rejects all instructions (the accounts are read-only from the perspective of
  transactions)
- Serves as the owner for the leader schedule and epoch stakes accounts
- Is updated exclusively by the runtime at epoch boundaries

### Runtime Behavior

#### Epoch Boundary Update

At each epoch boundary (when `parent.epoch() < new.epoch()`), the runtime:

1. Serializes the epoch stakes for `current_epoch + 1` (the vote account to
   stake mapping) and creates the account at `epoch_stakes(current_epoch + 1)`
2. Computes the leader schedule for `current_epoch + 1` using the same
   stake-weighted shuffle (`LeaderSchedule::new()`) that already populates the
   `LeaderScheduleCache`
3. Serializes the new schedule (embedding the SHA-256 hash of the epoch stakes
   written in step 1 as `epoch_stakes_hash`) and creates the account at
   `schedule(current_epoch + 1)`

Each newly created account is funded with the rent-exempt minimum and is never
modified again. There is no copying or rotation between addresses.

This integrates into the existing epoch-boundary processing in
`process_new_epoch()`, after vote account stake snapshots are taken and
`update_epoch_stakes()` has been called.

#### Feature Activation

On the first epoch boundary after feature activation, the runtime creates the
accounts for the current and next epoch (if the next epoch's stakes are
already available). No historical accounts are backfilled — data for epochs
preceding feature activation is not available on-chain.

From that point forward, each epoch boundary writes exactly one new leader
schedule account and one new epoch stakes account, as described above.

Consumers **MUST** check that an account exists (using e.g. `getAccountInfo`)
before attempting to read it. Accounts for epochs prior to feature activation,
or for epochs further in the future than the current leader schedule epoch,
will not exist.

#### State Growth

Each epoch contributes approximately 424 KB of new on-chain account state
(~344 KB for the leader schedule + ~80 KB for the epoch stakes). With
approximately two epochs per day, annual growth is on the order of 310 MB.
This is a deliberate tradeoff: preserving all history simplifies consumer
logic, eliminates write amplification at epoch boundaries, and enables
retrospective analysis (such as skip rate computation) without requiring
off-chain archival.

Future SIMDs may introduce a pruning mechanism if long-term state growth
becomes a concern. Because each account is independently addressable by
epoch, pruning can be added without breaking the address scheme for the
remaining accounts.

#### Consistency

The leader schedule written to these accounts is identical to what
`LeaderScheduleCache` computes and what `getLeaderSchedule` returns over RPC.
The deterministic computation (ChaCha20 RNG seeded with epoch, stake-weighted
sampling) is unchanged.

### RPC

No changes to existing RPC methods are required by this proposal. The
`getLeaderSchedule` and `getSlotLeaders` methods continue to work as before.

However, once the leader schedule is available as account data, the existing
RPC endpoints become redundant. Client libraries could implement
`getLeaderSchedule` as client-side account reads and deserialization rather
than dedicated RPC calls. This opens a path toward eventually deprecating
these endpoints, consistent with the broader goal of reducing
validator-specific RPC surface area.

## Alternatives Considered

### Sysvar Accounts

The most natural approach would be to make these sysvar accounts, following
the pattern of `SlotHashes`, `StakeHistory`, etc. However, the sysvar
infrastructure carries significant overhead:

- **Hardcoded cache:** The `SysvarCache` struct has a fixed field per sysvar.
  Adding a new sysvar requires modifications to ~15 files across the runtime,
  program-runtime, syscalls, SVM, and test infrastructure.
- **Per-bank caching:** Every bank creation populates the sysvar cache. For
  accounts that change only at epoch boundaries, this is unnecessary overhead.
- **Serialization constraints:** Sysvars traditionally use bincode
  serialization. The leader schedule benefits from a raw binary layout for
  zero-copy on-chain access.

A system-managed account owned by a dedicated native program achieves the same
goals (runtime-controlled, read-only, well-known addresses) without coupling to
the sysvar cache infrastructure. Programs read the account data directly, just
as they would any other account.

### Geyser Plugin Interface

The Geyser plugin interface could be extended to emit leader schedule data
directly at epoch boundaries, without storing it in an account. However, Geyser
is a push-only interface: plugins receive notifications but cannot query the
validator for data on demand. A consumer that starts mid-epoch, reconnects after
a disconnect, or simply needs the current schedule at an arbitrary point in time
would have no way to retrieve it without maintaining its own state from the
stream origin.

Storing the schedule in an account solves this naturally. Any consumer can read
the account at any time via the existing accounts infrastructure (snapshots,
`getAccountInfo`, Geyser account notifications). This also avoids adding
request-response semantics to Geyser, which would conflict with the ongoing
effort to decouple RPC-adjacent functionality from the validator.

### Syscall

A syscall like `sol_get_slot_leader(slot) -> Pubkey` would be more efficient
for on-chain programs that only need to check individual slots. However, there
is no concrete on-chain use case that justifies it today — the primary demand
is from off-chain consumers who need subscription-based access. A syscall could
be proposed in a follow-up SIMD if on-chain demand materializes; the runtime
already has the data structures to support it.

### Epoch Stakes Only (No Leader Schedule)

Since the leader schedule is deterministically derived from the epoch stakes,
one could argue that only epoch stakes need to be on-chain. However, the
leader schedule computation (ChaCha20 stake-weighted shuffle with ~108,000
samples) is too expensive to run in a program — it would far exceed
transaction CU limits. Without the precomputed schedule on-chain, use cases
like permissionless skip rate cranking would be impossible. Off-chain
consumers could compute the schedule from epoch stakes, but this proposal
serves both audiences. The epoch stakes accounts (~80 KB each) are a modest
addition alongside the leader schedule accounts.

### Fixed-Seed Rolling Accounts (Previous / Current / Next)

An earlier draft of this proposal used three fixed-seed PDAs per account type
(`["previous_schedule"]`, `["current_schedule"]`, `["next_schedule"]`) and
rotated their contents at each epoch boundary. This was rejected in favor of
epoch-number-keyed seeds for several reasons:

- **Write amplification.** Rotation requires rewriting the same data under
  different addresses every epoch, producing three writes per account type per
  epoch instead of one.
- **Bounded history.** A rolling layout only ever exposes three epochs. Any
  consumer that needs older history (for retrospective skip rate analysis, for
  example) would need to maintain their own archive, defeating a core
  motivation of the proposal.
- **Ambiguity at epoch boundaries.** A `current_schedule` account has an
  implicit epoch binding that changes on every epoch boundary, creating a
  race between the runtime write and any consumer reading the account. With
  epoch-keyed addresses, the account for a given epoch is written once and
  is unambiguous.

Reviewers on the SIMD discussion (trent-nelson, brooksprumo, joncinque) all
converged on epoch-keyed addressing as the cleaner design.

### Single Combined Account per Epoch

Storing the leader schedule and epoch stakes for a given epoch in a single
account would reduce account count but increase per-account size. Keeping
them separate allows programs to load only what they need, reducing
per-transaction account data.

### Identity-Only Table (No Vote Addresses)

An alternative is to store only the 32-byte validator identity in each table
entry, halving the identity table size (~64 KB vs ~128 KB at current mainnet
parameters). However, the Solana runtime internally keys leader assignment on
the combination of identity and vote account. A single identity can operate
multiple vote accounts, each with its own stake. Without vote addresses in the
table, it is impossible to determine which vote account was assigned a given
slot, making per-vote-account skip rate computation infeasible. Since skip rate
analysis is a primary motivation for including the previous epoch, omitting
vote addresses would undermine that use case.

### u32 Indices

Using `u32` indices instead of `u16` would support over 4 billion unique leaders
but doubles the schedule portion of the account (~432 KB vs ~216 KB at current
epoch length). Given that `u16` supports 65,535 unique leaders — over 30x the
current validator count — the space savings are worthwhile.

## Impact

**Validator operators:** Validators will create two new accounts per epoch
boundary (~424 KB combined) after the feature is activated, contributing
approximately 310 MB per year of on-chain state growth. The accounts are
written once at each epoch boundary and never modified thereafter, adding
negligible overhead to epoch processing. No configuration changes are required.

**RPC providers:** No immediate changes. Existing `getLeaderSchedule` and
`getSlotLeaders` endpoints continue to function. Over time, client libraries
may shift to reading the on-chain accounts directly, reducing load on these
endpoints.

**Indexers and Geyser plugin operators:** This is the primary beneficiary.
Indexers can subscribe to the leader schedule program via `programSubscribe`
(or the Geyser equivalent) to receive leader schedule and stake distribution
updates at epoch boundaries, replacing RPC polling. Consumers that want only
a specific epoch can subscribe to the corresponding epoch-keyed PDA directly.

**On-chain program developers:** Programs can read the leader schedule and
epoch stakes accounts for any epoch since feature activation by deriving the
PDA from the epoch number. The binary formats support zero-copy access.
Concrete use cases include permissionless skip rate computation (combining a
past epoch's schedule with the slot history sysvar), stake-weighted
governance, and quorum verification.

**Core contributors:** This proposal introduces a new pattern for
runtime-managed accounts (non-sysvar, owned by a native program that rejects
all instructions). This pattern may be reused for other large,
infrequently-updated state that does not warrant the overhead of the sysvar
cache.

## Security Considerations

### Account Size and Growth

Leader schedule accounts are bounded at ~344 KB each and epoch stakes accounts
at ~80 KB each by the 2,000-validator admission cap. New accounts are created
at each epoch boundary (~424 KB combined), yielding approximately 310 MB of
annual state growth. This is a deliberate tradeoff discussed under [State
Growth](#state-growth); pruning may be introduced in a follow-up SIMD if
warranted.

### Capitalization Impact

Each epoch boundary mints the rent-exempt minimum for the two new accounts
(~3 SOL per epoch at current rent parameters). Over a year this amounts to
approximately 2,200 SOL transferred into rent-exempt account balances. These
lamports are not burned — they remain in the leader schedule program's
accounts indefinitely. If a future pruning SIMD is adopted, freed lamports
could be returned to the treasury or burned.

### Read-Only Guarantees

The accounts are owned by a native program that rejects all instructions.
There is no code path by which a transaction can modify them — the only
writer is the runtime itself, acting at epoch boundaries.

Because each epoch's accounts are addressed by a deterministic PDA, the set
of currently-reserved addresses is unbounded over time and cannot be
enumerated statically. The program ID itself is added to the reserved account
keys list to prevent any transaction from acquiring a write lock on the
program, and all accounts owned by this program inherit the rejection
behavior through the owner check. This is sufficient because:

1. No valid transaction can mutate an account whose owner program rejects
   every instruction.
2. The runtime's writes happen outside of transaction processing and are
   therefore unaffected by the account-lock scheduler.
3. Any validator that diverged from the deterministic computation would
   produce a different bank hash and fail consensus.

Combined, these provide the same integrity guarantee as sysvar accounts.

### Determinism

The leader schedule computation is deterministic (same epoch + same stakes =
same schedule). All validators will produce identical account contents for the
same epoch, ensuring consensus on account state.

## Future Work

The following items are explicitly out of scope for this SIMD but are
enabled by it and may be addressed in follow-up proposals:

- **Snapshot manifest slimming.** Once epoch stakes are available as on-chain
  account state, the duplicate copy currently stored in the snapshot manifest
  (and potentially the bank) could be removed, reducing snapshot size and
  simplifying bank construction. This was flagged by topointon-jump on the
  SIMD discussion as a natural follow-up.
- **State pruning.** If long-term growth becomes a concern, a future SIMD
  could define a maximum retention window (e.g. the most recent N epochs)
  and return rent to the treasury when old accounts are closed.
- **Leader schedule syscall.** A dedicated `sol_get_slot_leader(slot)`
  syscall would allow programs to query individual slots without loading a
  ~344 KB account. This is deferred because no concrete use case has been
  identified that justifies the added surface area, but the data is now
  available should one emerge.
- **On-chain stake visibility across consensus changes.** Post-Alpenglow,
  when epoch stakes become the canonical input and delegation trees are
  removed from snapshots, this SIMD's accounts provide a natural home for
  that data in permanent on-chain form.

## Backwards Compatibility

This proposal introduces new accounts and a new native program. It does not
modify any existing accounts, programs, sysvars, or RPC methods. There are no
backwards compatibility concerns.

Validators that have not activated the feature will not create or update these
accounts. Once the feature is activated network-wide, all validators will
maintain consistent account state.
