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
mapping) for the previous, current, and next epochs in on-chain accounts.
This enables subscription-based access for indexers, permissionless on-chain
skip rate computation, and a general-purpose on-chain view of the stake
distribution.

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
epoch.

**Leader span:** A contiguous group of slots assigned to a single leader
(currently 4 slots, i.e. `NUM_CONSECUTIVE_LEADER_SLOTS`). The schedule is
indexed by leader span rather than by individual slot.

**Epoch stakes account:** A system-managed account that stores the mapping
from vote account to delegated stake (in lamports) for a single epoch.

## Detailed Design

### Account Structure

Three accounts are maintained: one for the **previous epoch**, one for the
**current epoch**, and one for the **next epoch**. Each account contains a
self-describing binary layout with the identity table (including vote
addresses) and the schedule index array.

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
| **Total (3 accounts)** | | **~1.03 MB** |

The identity table uses 64 bytes per entry (32-byte identity + 32-byte vote
account). Note that `num_leaders` may exceed the number of unique validator
identities if any identity operates multiple vote accounts.

With `u16` indices, the identity table supports up to 65,535 unique entries.
At 65,535 entries × 64 bytes = 4 MB for the identity table alone. The
theoretical maximum account size is ~4.2 MB, well within the 10 MB account
data limit. With Alpenglow's cap of 2,000 voting validators, the practical
maximum is well below this.

**Note on index width:** This proposal uses `u16` indices (2 bytes) rather than
`u32` (4 bytes), saving 216 KB per account at current mainnet parameters. The
`u16` limit of 65,535 entries provides substantial headroom. If the network
were to exceed this limit, a future SIMD could introduce a new version with
wider indices.

### Epoch Stakes Account Structure

Three additional accounts store the epoch stakes: one for each of the
**previous**, **current**, and **next** epochs. Each account contains a
sorted mapping from vote account to delegated stake.

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
| **Total (3 accounts)** | | **~240 KB** |

The `epoch_stakes_hash` in the leader schedule header is the SHA-256 hash
of the epoch stakes account data for the corresponding epoch (excluding the
header). This allows consumers to cross-reference the two account types and
verify that the leader schedule was derived from the published stakes.

### Account Addresses

All six accounts live at well-known addresses derived as Program Derived
Addresses (PDAs) from the owning program:

```
previous_schedule     = PDA(program_id, ["previous_schedule"])
current_schedule      = PDA(program_id, ["current_schedule"])
next_schedule         = PDA(program_id, ["next_schedule"])
previous_epoch_stakes = PDA(program_id, ["previous_epoch_stakes"])
current_epoch_stakes  = PDA(program_id, ["current_epoch_stakes"])
next_epoch_stakes     = PDA(program_id, ["next_epoch_stakes"])
```

Using PDAs rather than vanity-ground addresses ensures the addresses are
deterministic and verifiable. The seeds are fixed strings — the account
**contents** rotate at epoch boundaries, not the addresses. This means
consumers subscribe to exactly six stable addresses.

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

1. Rotates both account sets: current -> previous, next -> current
2. Serializes the epoch stakes for `current_epoch + 1` (the vote account to
   stake mapping) and writes to `next_epoch_stakes`
3. Computes the leader schedule for `current_epoch + 1` using the same
   stake-weighted shuffle (`LeaderSchedule::new()`) that already populates the
   `LeaderScheduleCache`
4. Serializes the new schedule (with `epoch_stakes_hash` from step 2) and
   writes to `next_schedule`

Account lamport balances are set to the rent-exempt minimum (or 1 lamport,
whichever is greater) on each write.

This integrates into the existing epoch-boundary processing in
`process_new_epoch()`, after vote account stake snapshots are taken and
`update_epoch_stakes()` has been called.

#### Feature Activation

On the first epoch boundary after feature activation:

1. All six accounts are created with the rent-exempt balance (minimum 1
   lamport, since zero-lamport accounts are treated as non-existent by the
   runtime)
2. Previous accounts (`previous_schedule`, `previous_epoch_stakes`) are left
   empty (no prior epoch data is available)
3. Current accounts are populated with the current epoch's data
4. Next accounts are populated with the next epoch's data, if vote account
   stakes for that epoch are available. If not yet available, the next
   accounts are left empty and will be populated at the next epoch boundary

Consumers **must** check the `epoch` field in the header before using the
account data. An empty account (zero data length) indicates that no schedule
is available for that slot yet.

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

### Two Epochs (Current, Next)

An earlier draft of this proposal used only two accounts (current + next). Three
epochs were chosen instead because the previous epoch's schedule is required
for permissionless on-chain skip rate computation. Without the previous epoch
available on-chain, consumers would need to maintain their own off-chain
schedule history, which undermines the self-contained nature of this proposal.
The additional ~344 KB of state is a modest cost for enabling this use case.

### Single Combined Account with Both Epochs

Storing both epochs in one account would halve the number of accounts but
roughly double the account size. Separate accounts allow programs to load only
the epoch they need, reducing per-transaction account data.

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

**Validator operators:** Validators will create and maintain six additional
accounts (~1.27 MB total) after the feature is activated. The accounts are
updated once per epoch boundary, adding negligible overhead to epoch
processing. No configuration changes are required.

**RPC providers:** No immediate changes. Existing `getLeaderSchedule` and
`getSlotLeaders` endpoints continue to function. Over time, client libraries
may shift to reading the on-chain accounts directly, reducing load on these
endpoints.

**Indexers and Geyser plugin operators:** This is the primary beneficiary.
Indexers can subscribe to six stable account addresses to receive leader
schedule and stake distribution updates at epoch boundaries via Geyser or
websocket `accountSubscribe`, replacing RPC polling.

**On-chain program developers:** Programs can read the leader schedule and
epoch stakes accounts to access the schedule or stake distribution for the
previous, current, or next epoch. The binary formats support zero-copy access.
Concrete use cases include permissionless skip rate computation (combining the
previous epoch's schedule with the slot history sysvar), stake-weighted
governance, and quorum verification.

**Core contributors:** This proposal introduces a new pattern for
runtime-managed accounts (non-sysvar, owned by a native program that rejects
all instructions). This pattern may be reused for other large,
infrequently-updated state that does not warrant the overhead of the sysvar
cache.

## Security Considerations

### Account Size

Leader schedule accounts are ~344 KB each and epoch stakes accounts are ~80 KB
each at current mainnet parameters. The combined footprint of ~1.27 MB for all
six accounts is modest relative to overall validator memory usage and well
within per-account limits.

### Capitalization Impact

Creating these accounts at feature activation increases total capitalization by
the rent-exempt minimum for ~1.27 MB of account data. At current rent
parameters this is approximately 9 SOL. This is a one-time, small increase
that occurs at the epoch boundary when the feature activates. No ongoing
lamport changes occur beyond minor adjustments if account sizes change between
epochs.

### Read-Only Guarantees

The accounts are protected by two independent mechanisms:

1. **Program-level:** The owning native program rejects all instructions, so no
   transaction can modify the accounts through program invocation.
2. **Transaction-level:** The program ID and all six PDA addresses are added
   to the reserved account keys list (gated on the same feature). This prevents
   any transaction from acquiring a write lock on these accounts, even if a
   malicious program were to claim ownership.

Combined, these provide the same integrity guarantee as sysvar accounts.

### Determinism

The leader schedule computation is deterministic (same epoch + same stakes =
same schedule). All validators will produce identical account contents for the
same epoch, ensuring consensus on account state.

## Backwards Compatibility

This proposal introduces new accounts and a new native program. It does not
modify any existing accounts, programs, sysvars, or RPC methods. There are no
backwards compatibility concerns.

Validators that have not activated the feature will not create or update these
accounts. Once the feature is activated network-wide, all validators will
maintain consistent account state.
