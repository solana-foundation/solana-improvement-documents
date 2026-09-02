---
simd: '0649'
title: Priority Ordering Within Entry Batches
authors:
  - Max Resnick (Anza)
category: Standard
type: Core
status: Draft
created: 2026-09-02
feature: TBD
---

## Summary

Enforce transaction priority order within each entry batch as a block validity
rule.

A leader MUST record the transactions of every entry batch in
non-increasing priority order, where priority is the fee-per-cost score that
schedulers use to rank transactions. Replay checks the order and
marks a block that violates this rule as invalid.

To keep the rule from being vacuous, every entry batch other than the last
one in a block MUST span at least two FEC sets.

Originating discussion: [Discussion #605].

[Discussion #605]: https://github.com/solana-foundation/solana-improvement-documents/discussions/605

## Motivation

The original vision for Solana was a decentralized NASDAQ, and that remains
the goal. An exchange must be fast and cheap, so increasing bandwidth and
reducing latency is essential. But performance alone is necessary, not
sufficient. An exchange must also be fair, deterministic, and predictable,
with a market structure that participants can inspect and reason about.
Without these properties it is much harder to onboard new traders and to
obtain favorable regulatory outcomes.

At least 13 distinct scheduler implementations are active on mainnet today,
with more coming. Reasoning about scheduling was hard enough when nearly
every leader ran the same software. With this much diversity it is
increasingly difficult to know how any given leader will prioritize
transactions, resolve conflicts, form batches, and execute them. Tools that
help traders understand leader behavior have become hard to keep current,
and even sophisticated on-chain traders with strong incentives to stay
current are having trouble keeping up.

Scheduler experimentation has produced valuable performance work and exposed
many transaction-pipeline bugs. Performance, however, does not require
leaving the order of transactions within an entry batch to scheduler
discretion. A scheduler can still decide what to include and how to form
batches; it just has to write each batch down in priority order.

Enforcing the order in replay makes the rule hold for every client and every
scheduler, and the check is cheap: one integer comparison per transaction
next to the signature verification that replay already performs.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0317]: Enforce 32 data + 32 coding shreds**

    Fixed FEC set sizes give "two FEC sets" a fixed meaning: 64 data shreds.

- **[SIMD-0337]: Markers for Alpenglow Fast Leader Handover**

    Section "`DATA_COMPLETE_SHRED` Placement Rules" requires that an entry
    batch can only end on the final data shred of an FEC set. The same rule
    appears as check 9 of [SIMD-0504]. This proposal relies on that
    alignment; if neither is active first, the alignment rule MUST be
    enforced as part of this feature.

This proposal uses the terms *entry batch* and *block marker* as defined in
[SIMD-0307]. It does not depend on the block footer itself.

[SIMD-0307]: https://github.com/solana-foundation/solana-improvement-documents/pull/307
[SIMD-0317]: https://github.com/solana-foundation/solana-improvement-documents/pull/317
[SIMD-0337]: https://github.com/solana-foundation/solana-improvement-documents/pull/337
[SIMD-0504]: https://github.com/solana-foundation/solana-improvement-documents/pull/504

## New Terminology

- **Entry batch**: the array of entries deserialized from the data shreds
  between two consecutive batch boundaries. A batch boundary is a data shred
  with the `DATA_COMPLETE_SHRED` flag set (the `LAST_SHRED_IN_SLOT` flag
  implies it). A block marker occupies a batch slot but is not an entry
  batch.
- **Batch order**: the order of transactions within an entry batch as
  recorded in the ledger: entries in order, and within each entry,
  transactions in order. This is the order replay executes today.
- **Requested cost units**: the pre-execution cost model estimate of a
  transaction, in compute units. It is the value block packing and block
  cost limit accounting use before a transaction executes, computed from the
  requested compute unit limit and the requested loaded accounts data size
  limit rather than the consumed values. This is the same quantity named in
  [SIMD-0553].
- **Leader reward**: the lamports credited to the leader for including a
  transaction under the fee rules active in the bank. Today this is the
  prioritization fee plus the un-burned half of the base fee.
- **Priority**: the integer score defined in "Priority" below.
- `PRIORITY_MULTIPLIER = 1_000_000`.
- `MIN_FEC_SETS_PER_ENTRY_BATCH = 2`.

[SIMD-0553]: https://github.com/solana-foundation/solana-improvement-documents/pull/553

## Detailed Design

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC
2119](https://www.ietf.org/rfc/rfc2119.txt) and [RFC
8174](https://www.ietf.org/rfc/rfc8174.txt).

### Overview

Two new block validity rules are introduced, both checked in replay:

1. **Ordering rule**: within every entry batch, non-exempt transactions
   appear in non-increasing priority order.
2. **Minimum batch size rule**: every entry batch other than the final entry
   batch of the block spans at least `MIN_FEC_SETS_PER_ENTRY_BATCH` FEC
   sets.

A block that violates either rule is invalid. Validators MUST NOT vote for
it and MUST treat it the same way they treat a block that exceeds the block
cost limits today (for example, by marking the slot dead). Nothing about
execution changes: replay executes a valid block in batch order exactly as
it does today.

In pseudocode, the change to processing a complete entry batch is:

```text
today:
    replay(batch.transactions)                    // order recorded by leader

with this proposal:
    if not is_batch_size_valid(batch):
        return InvalidBlock
    if not is_sorted_by_priority_desc(batch.transactions):
        return InvalidBlock
    replay(batch.transactions)                    // unchanged
```

### Priority

For a sanitized transaction `tx` replayed in bank `bank`:

```text
cost     = requested_cost_units(tx, bank)
reward   = leader_reward(tx, bank)
priority = saturating_mul(reward, PRIORITY_MULTIPLIER) / (cost + 1)
```

All arithmetic is unsigned 64-bit. Division is integer division. The `+ 1`
in the denominator only guards against division by zero; real costs are
never below several hundred compute units, so it does not affect ordering
in practice.

`requested_cost_units` is the sum of the cost model components that block
packing uses to reserve block space before execution: signature costs, write
lock cost, instruction data bytes cost, program execution cost computed from
the requested compute unit limit, and loaded accounts data size cost
computed from the requested loaded accounts data size limit. It MUST be the
same value the implementation uses for pre-execution block cost limit
accounting.

`leader_reward` is the number of lamports the leader is credited for the
transaction under the fee rules active in `bank`. With today's rules that
is:

```text
base_fee     = lamports_per_signature * num_signatures   // incl. precompiles
burn         = base_fee * 50 / 100
leader_reward = prioritization_fee + (base_fee - burn)
```

If a later proposal changes the fee split (for example [SIMD-0553]),
`leader_reward` follows the new split automatically. Priority is defined in
terms of the leader's reward so that the canonical order tracks the leader's
own incentive.

Priority depends only on the sanitized transaction, the addresses resolved
from any address lookup tables against `bank`, and the fee parameters and
feature set of `bank`. It does not depend on account state beyond lookup
table resolution and does not depend on execution results, so it can be
computed before, during, or after execution with the same result.

This is the same score the Agave scheduler uses today to rank transactions
(`calculate_priority_and_cost`). Making it a protocol quantity means every
client MUST compute it identically, in the same way the cost model is
already a protocol quantity through the block cost limits.

### Ordering rule

Let `t_1, ..., t_n` be the non-exempt transactions of an entry batch in
batch order. The batch is valid only if:

```text
priority(t_i, bank) >= priority(t_(i+1), bank)    for all 1 <= i < n
```

Transactions with equal priority MAY appear in any relative order.

**Exempt transactions.** Simple vote transactions, using the same
classification that the block-level vote cost limit uses, are exempt. They
are skipped when forming the sequence `t_1, ..., t_n` and MAY appear
anywhere in the batch. Votes do not compete with user transactions for
account state and their placement is driven by latency rather than fees;
forcing them into the fee order would delay votes without improving
fairness. All other transactions, including a leader's own transactions,
are subject to the rule.

**Streaming.** The rule compares adjacent transactions only, so a validator
does not need the whole batch before it can begin checking or executing.
Implementations MAY deserialize and execute transactions of a partially
received batch, comparing each transaction against its predecessor as it
arrives, as long as a violation discovered later causes the block to be
treated as invalid. This is the same situation as a malformed transaction
appearing late in a batch today.

**No reordering.** Replay MUST NOT reorder transactions to repair an invalid
batch. The recorded order is canonical. RPC methods such as `getBlock`
continue to return transactions in the recorded order.

### Minimum batch size rule

Let the FEC-aligned batch boundary rule from [SIMD-0337] (equivalently,
check 9 of [SIMD-0504]) be in force, so every entry batch begins at the
first data shred of an FEC set and ends at the last data shred of an FEC
set. Then, for every entry batch that is not the final entry batch of the
block:

```text
num_data_shreds(batch) >= MIN_FEC_SETS_PER_ENTRY_BATCH * 32
```

With fixed 32-data-shred FEC sets this is exactly "spans at least two FEC
sets". Block markers are not entry batches and are not subject to this
rule. The final entry batch of the block, meaning the last batch that
contains entries, is exempt because the leader must close it when the slot's
ticks are exhausted regardless of how much data it holds. It remains
subject to the ordering rule.

Without this rule a leader could make every entry batch a single
transaction and the ordering rule would constrain nothing. Two FEC sets
(64 data shreds, about 63 KB of entry data) is what both Agave's broadcast
stage and Firedancer's shred tile already target when forming entry
batches, so the rule codifies current practice. Implementations that
currently close a batch early, for example on a coalescing timeout or when
a batch happens to pack tightly into one FEC set, MUST instead keep
accumulating entries until the batch reaches two FEC sets or the slot ends.

### Leader behavior

Leaders MUST produce blocks that satisfy both rules. How they do so is not
prescribed. Any scheduling strategy, hardware configuration, or execution
model is acceptable as long as the recorded output is valid.

In practice a leader builds each entry batch as a window: it collects the
transactions it will include in the batch, sorts them by priority, forms
entries from the sorted sequence, and records them. A transaction that
arrives after a window's contents have been fixed goes into a later window.
Latency-sensitive leaders can keep windows short by keeping batches close
to the two-FEC-set minimum.

A leader that executes transactions before recording them MUST make sure
that any two conflicting transactions in the same batch were executed in
the order they are recorded; otherwise the leader's locally computed state
will differ from replay. Non-conflicting transactions produce the same state
in any order, so they MAY be executed in whatever order is convenient and
recorded in priority order. Agave's scheduler already dispatches conflicting
transactions to workers in priority order, so the main change on the leader
side is buffering a window's results and recording them sorted rather than
in completion order.

A leader MAY also skip local execution entirely and leave execution to
replay, as long as the block it records is valid in batch order.

### Non-goals

- This does not constrain inclusion and does not provide slot-global
  ordering. Leaders still choose which transactions enter each entry batch
  and may defer a transaction to a later batch.
- This does not prescribe leader execution, replay parallelism, or new block
  limits.
- This does not prevent a leader from prioritizing its own transactions by
  paying itself priority fees. Because prioritization fees go entirely to
  the leader ([SIMD-0096]), a leader can give its own transactions arbitrary
  priority at the cost of only the burned share of the base fee. Addressing
  that requires further changes.

[SIMD-0096]: https://github.com/solana-foundation/solana-improvement-documents/pull/96

### Edge Cases

- **Entries without transactions.** Tick entries contribute nothing to the
  sequence and are ignored by the ordering rule.
- **Batches with no non-exempt transactions.** Trivially valid.
- **Block markers.** A block marker ([SIMD-0307], [SIMD-0337]) is not an
  entry batch. It is subject to neither rule and does not count toward the
  size of an adjacent entry batch. When a block footer occupies the last
  batch slot, the final entry batch is the last batch containing entries,
  which precedes the footer.
- **Interrupted slots.** If a leader abandons a slot and emits a closing
  batch to mark it, that closing batch is the final batch and is exempt from
  the minimum size rule.
- **Low throughput.** If fewer than two FEC sets of entries are produced in
  a whole slot, the block is a single final entry batch, which is valid.
- **Transactions that fail.** A transaction that later fails to load or
  execute still has a well-defined priority from its message and is still
  part of the sequence.
- **Saturation.** `reward * PRIORITY_MULTIPLIER` uses saturating
  multiplication. Realistic fees are far below the saturation point, but
  implementations MUST saturate identically.
- **Equal priorities.** Any relative order is valid. Implementations MUST
  use `>=`, not `>`.
- **Feature boundary.** The rules apply to blocks in slots whose bank has
  the feature active. Blocks produced before activation are not checked.
- **Sanitization failures.** A batch containing a transaction that fails
  sanitization is already invalid today; the ordering check is only defined
  over sanitized transactions.

### Validator Components Affected

<!-- markdownlint-disable MD013 -->
| Validator Component             | Impact                                                                  |
|---------------------------------|-------------------------------------------------------------------------|
| Transaction Execution (Runtime) | None. Execution order is unchanged.                                     |
| Virtual Machine                 | None.                                                                   |
| Block Packing                   | Must record each entry batch in priority order and honor the minimum batch size. |
| Consensus                       | Replay adds two validity checks; violating blocks are invalid.          |
| Gossip                          | None.                                                                   |
| Turbine                         | Broadcast must not close a non-final entry batch before two FEC sets.   |
| Snapshots                       | None.                                                                   |
| On-Chain Core BPF Programs      | None.                                                                   |
| Other (RPC)                     | None. `getBlock` order is the recorded order, as today.                 |
<!-- markdownlint-enable MD013 -->

## Alternatives Considered

**Full in-protocol ordering across the whole slot.** This is the long-term
direction, but it requires constraining inclusion and would remove the
pipelining that lets leaders stream a block while building it. Ordering
within batches is a step that is compatible with current block production.

**Have replay reorder transactions instead of rejecting the block.** Replay
would then execute a different order than the leader did, so a leader that
executes locally could not know the resulting state, and revert protection
and other execution-dependent leader behavior would break. Reordering also
requires the whole batch before any execution can start. Rejecting keeps
the leader's recorded order canonical.

**Use compute unit price alone as the priority.** Simpler for users to
reason about, but it ignores the base fee contribution and the cost model,
ranks all zero-price transactions equally, and diverges from the score
schedulers use today. The ordering check itself is agnostic to the priority
function, so the definition can be revisited in a later proposal without
changing the enforcement mechanism.

**Enforce order per entry rather than per entry batch.** An entry is a set
of non-conflicting transactions sized for parallel execution and is chosen
freely by the leader, so per-entry ordering would be trivially satisfiable
and constrain nothing.

**Define the ordering window by fixed FEC set index ranges instead of
batches.** Transactions can straddle FEC set boundaries, so this would need
a rule for which window a straddling transaction belongs to. Entry batches
are already the deserialization unit and are already aligned to FEC sets;
adding a minimum batch size achieves the same goal with less machinery.

**A different minimum batch size.** One FEC set (about 31 KB) is small
enough that batches on a busy leader would hold only a few dozen
transactions, which is easy to game by splitting. Larger minimums increase
leader-side latency at low throughput. Two FEC sets matches what both major
clients already produce. The constant can be revisited.

## Impact

**Traders and application developers** get a canonical, inspectable order
within each entry batch: a transaction's position is determined by its
priority relative to the other transactions in its batch, independent of
which client or scheduler the leader runs. Tooling that predicts or audits
ordering becomes simpler to build and maintain. Clients still compete on
inclusion, latency, and batch formation.

**Validators** running a leader implementation must sort each batch before
recording it and must not close non-final batches before two FEC sets. In
Agave this means buffering a window's execution results and recording them
in priority order rather than in completion order. Replay-side cost is one
integer comparison per transaction, negligible next to signature
verification.

**Bundles.** Bundle-like behavior remains possible. A leader controls which
transactions enter a batch, so it can keep a bundle's transactions adjacent
by not admitting transactions with intermediate priority into the same
batch. Bundle payment must be expressed in a way that yields the intended
priority order, for example as prioritization fees on the bundle's own
transactions rather than a separate tip transaction placed at an arbitrary
position. Revert protection is still possible for leaders that execute
locally.

The longer-term plan is to replace bundles with in-protocol features.
Bundles provide two things: all-or-nothing execution and revert protection.
Rising transaction size limits let many benign all-or-nothing use cases fit
in a single transaction. For revert protection, revenue equivalence suggests
that an all-pay auction (ordinary priority fees) and a first-price auction
(revert-protected fees) raise similar revenue, with back-runs as the likely
exception. An in-protocol success or failure fee may make sense later, but
it complicates the path to in-protocol ordering and is best considered after
that goal is reached.

**Core contributors** must treat the priority function as a consensus
quantity with shared conformance vectors, in the same way the cost model is
treated today.

## Security Considerations

**Cross-client divergence.** The priority function must be bit-identical
across clients; any divergence in the cost model, fee calculation, or the
rounding of the score would cause one client to reject blocks another
accepts. The function is built entirely from quantities that are already
consensus-critical (fees and requested cost units), which limits new
surface area.

**Leader self-dealing.** A leader can pay itself prioritization fees to give
its own transactions any position. The cost to the leader is the burned
share of the base fee only. This proposal does not change that.

**Boundary and padding games.** A leader can still defer a transaction to a
later batch, or fill a batch with its own low-value transactions to close it
early. Both consume the leader's fee budget and block space and leave
visible traces in the ledger. This proposal does not try to
prevent them from doing this.

**Denial of service.** The check is linear in the number of transactions
and needs no account loading beyond lookup table resolution, which replay
already performs.

**Skipped slots during rollout.** A leader that produces unordered or
undersized batches after activation produces invalid blocks and loses its
slots. Activation should wait until all clients with meaningful stake
produce compliant blocks. Producing compliant blocks before activation is
harmless, so leader-side changes can ship first.

## Drawbacks

- It constrains scheduler design. A scheduler can no longer record a
  transaction as soon as it executes; it must fix a window's contents,
  sort, and then record.
- A high-priority transaction that arrives after its window closes lands in
  the next window rather than at the front of the current one. With batches
  near the two-FEC-set minimum, this delay is on the order of tens of
  milliseconds on a busy leader.
- The minimum batch size adds broadcast latency when throughput is low,
  because a leader cannot ship a non-final batch until two FEC sets of
  entries exist or the slot ends. Both major clients already target this
  size, so the marginal change is small.
- It adds two more ways for a block to be invalid, which increases the
  importance of cross-client conformance testing.

## Backwards Compatibility

The change is gated by a feature. Before activation, replay does not check
either rule and blocks are processed as today. After activation, blocks that
violate either rule are invalid, so leaders running software that does not
order batches or that closes batches early will have their blocks rejected.
Transaction formats, RPC responses, and execution semantics are unchanged.
  