---
simd: '0525'
title: Reduce Slot Times
authors:
  - Brennan Watt (Anza)
category: Standard
type: Core
status: Draft
created: 2026-05-01
feature:
  - 350ms - iBRL2iJvhLssJveF1utbmmQGmjonmNYZALcJFEHTbUF
  - 300ms - iBRLA3zvd6x9445cK1vS7xt8n6Y7DS3otfRcDdW8JRW
  - 250ms - iBRLR6nG3fDi8YD4mpPTUVTgo5NaiYfZgrzokCfADP2
  - 200ms - iBRLypKvvj9VEvwoTeRpbLhbW55NFR4T3GE9BUR8A16
---

## Summary

Reduce Solana's target slot time from 400ms to 200ms in four feature-gated
steps:

1. 350ms slots
2. 300ms slots
3. 250ms slots
4. 200ms slots

Each step keeps `ticks_per_slot` fixed at 64, keeps leader windows fixed at 4
slots, and keeps epochs fixed at 432,000 slots. Per-slot work limits are reduced
in proportion to the target slot time so the corresponding wall-clock rate
remains approximately unchanged. `slots_per_year` is increased by the inverse
ratio so inflation remains unchanged from a wall-clock perspective. Precise
values are called out below.

Each feature gate becomes effective one epoch after activation. For example, a
feature that gets applied during transition from epoch N to N+1 takes effect for
slots starting in epoch N+2. This is so Turbine can cleanly apply the reduced
shred limits.

## Motivation

Shorter slots reduce confirmation and finalization latency for users. Any
consensus or commitment threshold measured in slots takes less wall-clock time
as slot time decreases.

Keeping the leader span at 4 slots also shortens the wall-clock duration of a
single leader window. A leader currently controls a nominal 1.6s window. At
200ms slots that window is 800ms. This improves market structure by reducing
the worst-case time a leader can delay, reorder, or selectively include
transactions before the next leader has an opportunity to produce a block.

Shorter slots also provide a finer-grained on-chain understanding of time.
Applications that interpret freshness in slots, such as oracle consumers and
propAMM-style market makers, can make decisions with a smaller slot-time
quantization error.

The staged rollout is intended to discover timing, implementation, and
operational problems at 350ms, 300ms, and 250ms before reaching 200ms. This lets
client teams verify that the slot, leader-window, and epoch timing assumptions
continue to close without changing the leader schedule or epoch schedule at the
same time.

## New Terminology

No new protocol terminology is introduced.

This proposal refers to the following placeholder feature gates. Final feature
IDs are TBD:

- `reduce_slot_time_to_350ms`
- `reduce_slot_time_to_300ms`
- `reduce_slot_time_to_250ms`
- `reduce_slot_time_to_200ms`

## Detailed Design

Each feature gate selects a new target slot duration. Implementations MUST use
the effective target slot duration for the bank being produced or verified. If
multiple slot-time feature gates are effective, implementations MUST select the
lowest effective target slot duration.

### Feature Activation Delay

Feature activation and feature effectiveness are separate for this proposal. A
slot-time feature gate MUST take effect on a one-epoch delay. If a gate first
becomes active in epoch `E`, all slots in epoch `E` MUST continue using the
previous effective slot-time parameters, and the new target slot duration and
derived limits MUST first apply at the first slot of epoch `E + 1`.

This delay is required because these gates reduce the maximum data and coding
shreds per slot. Turbine and shred-fetch filtering can enforce shred limits
outside the bank execution path, so they cannot rely on an execution-time
feature transition being perfectly synchronized with block production and
validation. The one-epoch delay gives validators an epoch of advance notice
before network shred filtering starts enforcing the reduced limits.

The feature gates SHOULD be activated in order. Activating them in order is not
required for value derivation, however. Integer limits are normative table
values and MUST be assigned directly or derived from the original 400ms baseline
values. Implementations MUST NOT repeatedly scale already-rounded values from a
previous stage, because that can accumulate rounding error.

### Timing Values

The `slots_per_year` values below use the Mainnet-Beta bank value of
`78_892_314.984`, as the 400ms baseline. Implementations MUST scale the bank's
stored `slots_per_year` value according to the table below at each effective
slot-time transition. `rent_collector.slots_per_year` MUST be updated with the
same value.

| Target | Feature gate |
|--------|--------------|
| 400ms | current |
| 350ms | `reduce_slot_time_to_350ms` |
| 300ms | `reduce_slot_time_to_300ms` |
| 250ms | `reduce_slot_time_to_250ms` |
| 200ms | `reduce_slot_time_to_200ms` |

| Target | ns/slot | tick ns | span | epoch | slots/year |
|--------|---------|---------|------|-------|------------|
| 400ms | 400000000 | 6250000 | 1.6s | 48h | 78892314.984 |
| 350ms | 350000000 | 5468750 | 1.4s | 42h | 90162645.696 |
| 300ms | 300000000 | 4687500 | 1.2s | 36h | 105189753.312 |
| 250ms | 250000000 | 3906250 | 1.0s | 30h | 126227703.974 |
| 200ms | 200000000 | 3125000 | 0.8s | 24h | 157784629.968 |

Note that none of the above table values (except for `slots_per_year` for
inflation) are explicitly in protocol and may deviate from reality, but they are
expected to be useful for ensuring expected timing is understood.

### Scaled Per-Slot Values

Integer values are scaled by `trunc(current_400ms_value * target_slot_ms /
400)`. Truncating is used whenever the proportional value is fractional so the
new value does not exceed the intended wall-clock budget.

| Limit | 400ms | 350ms | 300ms |
|-------|-------|-------|-------|
| Hashes/tick | 62500 | 54687 | 46875 |
| Max block CUs | 60000000 | 52500000 | 45000000 |
| Max writable acct CUs | 24000000 | 21000000 | 18000000 |
| Max vote CUs | 36000000 | 31500000 | 27000000 |
| Max data delta | 100000000 | 87500000 | 75000000 |
| Max data shreds | 32768 | 28672 | 24576 |
| Max coding shreds | 32768 | 28672 | 24576 |
| PER stake writes | 4096 | 3584 | 3072 |

| Limit | 250ms | 200ms |
|-------|-------|-------|
| Hashes/tick | 39062 | 31250 |
| Max block CUs | 37500000 | 30000000 |
| Max writable acct CUs | 15000000 | 12000000 |
| Max vote CUs | 22500000 | 18000000 |
| Max data delta | 62500000 | 50000000 |
| Max data shreds | 20480 | 16384 |
| Max coding shreds | 20480 | 16384 |
| PER stake writes | 2560 | 2048 |

The `hashes_per_tick` value MUST NOT be reduced by these feature gates when
Alpenglow is active. Alpenglow's low-power PoH path should keep its active
hashing behavior unchanged, currently expected to be one hash per tick.

If another feature gate changes one of the 400ms baseline limits before this
SIMD activates, the slot-time feature implementation MUST compose with that
feature by applying the same ratios to the active 400ms baseline. For example,
if a future block-limit feature changes `MAX_BLOCK_UNITS`, the slot-time stages
must use the new block limit multiplied by the target slot-time ratio.

For 100M CUs feature (SIMD-0286), the limits would look like so (anything not
mentioned below would match the tables above):
| Limit | 400ms | 350ms | 300ms |
|-------|-------|-------|-------|
| Max block CUs | 100000000 | 87500000 | 75000000 |
| Max writable acct CUs | 40000000 | 35000000 | 30000000 |

| Limit | 250ms | 200ms |
|-------|-------|-------|
| Max block CUs | 62500000 | 50000000 |
| Max writable acct CUs | 25000000 | 20000000 |

### Runtime Changes

When a slot-time feature gate becomes effective for a bank after the one-epoch
delay, the bank and its descendants MUST use the target values above for:

- `ns_per_slot`
- `slots_per_year`
- `rent_collector.slots_per_year`
- non-Alpenglow `hashes_per_tick`
- cost tracker block limits
- block accounts data size delta
- broadcast and shred-fetch shred limits
- partitioned epoch rewards stake-account stores per block

Snapshot restore and bank deserialization MUST reconstruct all non-persisted
runtime values from the effective slot-time stage and the bank's slot. A
validator restarting after activation or after an effective transition must
produce and validate the same limits as a validator that remained online across
the transition.

Snapshot serialization MUST also persist the effective timing values in the
snapshot manifest. This proposal does not add new manifest fields, but the
existing `ns_per_slot`, `hashes_per_tick`, and `slots_per_year` manifest values
MUST change in snapshots taken at or after a slot-time feature's effective slot.
Snapshots taken before the effective slot, including during the one-epoch delay
after activation, MUST keep the previous effective values. For non-Alpenglow
banks, snapshot manifests MUST serialize the following values:

| Effective target | `ns_per_slot` | `hashes_per_tick` |
|------------------|---------------|-------------------|
| 400ms | 400000000 | 62500 |
| 350ms | 350000000 | 54687 |
| 300ms | 300000000 | 46875 |
| 250ms | 250000000 | 39062 |
| 200ms | 200000000 | 31250 |

| Effective target | `slots_per_year` |
|------------------|------------------|
| 400ms | 78892314.984 |
| 350ms | 90162645.696 |
| 300ms | 105189753.312 |
| 250ms | 126227703.974 |
| 200ms | 157784629.968 |

When Alpenglow is active, the `hashes_per_tick` manifest value MUST retain
Alpenglow's active hashing behavior (set to `None`) instead of using the reduced
table value.

Inflation calculations MUST account for slot ranges that cross one or more
effective slot-time transitions. Historical slots before an effective transition
use the previous `slots_per_year`; slots at or after the effective transition
use the new
`slots_per_year`. This preserves issuance from a wall-clock perspective even
though the number of slots per epoch remains unchanged.

Shred validation MUST be slot-aware. Shreds for slots before a feature's
effective slot are validated with the previous shred limit, including shreds in
the one-epoch delay period after activation. Shreds for slots at or after the
effective slot are validated with the newly effective limit.

### Notable Non-Changes

This proposal intentionally does not change:

- Leader span: still 4 slots.
- Ticks per slot: still 64.
- Epoch length: still 432,000 slots.
- `solana-clock` and SDK constant values. Agave may dynamically scale bank
  values, but `solana-sdk` constants remain unchanged initially.
- Grace ticks.
- Timely Vote Credits grace.
- Repair delay, including the 250ms repair defer threshold.
- Vote costs.
- `fee_rate_governor.target_signatures_per_slot`.
- Blockhash queue and status cache max entries.
- Gossip `DEFAULT_MS_PER_SLOT` heuristics, including the CRDS entry
  eviction/retention window, the freshness threshold for accepting pull
  responses, and the scoring policy for which CRDS values to include in pull
  responses.

Note that some of these are out of protocol (Agave client defaults), but are
included here for completeness.

### Edge Cases

Implementations should pay special attention to:

- Feature gates activated at genesis, where the feature is active from slot 0
  but the reduced target slot time is not effective until epoch 1 unless genesis
  explicitly defines the reduced target as the baseline.
- Feature gates activated after snapshots, where runtime-only values must be
  rebuilt during restore.
- Multiple active slot-time gates, where the lowest target slot time whose
  one-epoch delay has elapsed wins.
- The activation epoch for each slot-time gate, where bank execution, block
  packing, broadcast, shred-fetch, and shred validation must continue using the
  previously effective limits.
- Alpenglow activation, where `hashes_per_tick` must not be reduced by these
  feature gates.
- Integer rounding, especially the 350ms and 250ms `hashes_per_tick` values.
- Inflation calculations for epochs and rewards that span effective transition
  slots.
- Shreds received around activation and effective-transition boundaries, which
  must be validated using the effective slot-time stage for the shred's slot.

### Validator Components Affected

- Transaction Execution (Runtime): Bank timing fields, inflation slot-to-time
  conversion, cost tracker, block data size limits, and PER distribution limits
  change by feature gate.
- Virtual Machine: No direct VM semantic change. Programs may observe finer
  slot-time granularity through sysvars and RPC-derived timing.
- Block Packing: Leaders must pack smaller per-slot CU, account CU, vote CU,
  accounts-data, shred, and PER write budgets.
- Consensus: Feature-gated slot duration changes are consensus critical. Leader
  windows remain 4 slots but become shorter in wall-clock time.
- Gossip: Vote and epoch-slot traffic increase per wall-clock time as slots get
  faster. Gossip uses of `DEFAULT_MS_PER_SLOT` to derive epoch duration for CRDS
  entry retention, pull-response freshness, and pull-response value scoring
  continue to use 400ms for the slot time portion of these calculations.
- Turbine: The per-slot data and coding shred limits are reduced
  proportionally after the one-epoch effectiveness delay, so shred filtering and
  bank execution enforce the same slot-aware limits.
- Snapshots: Snapshot manifests must persist the effective `ns_per_slot`,
  `hashes_per_tick`, and `slots_per_year` values, and snapshot restore must
  reconstruct effective runtime limits. Snapshot interval policy is not changed
  by this proposal.
- On-Chain Core BPF Programs: No direct program interface change. Programs that
  interpret slot distance as wall-clock time may need updates.
- Other: SDKs, RPC clients, explorers, and off-chain systems that use static
  slot-time constants will temporarily disagree with chain reality.

## Alternatives Considered

### Change Slots Per Leader Span

One alternative is to increase the number of slots in each leader span as slot
time decreases, keeping the leader window close to the current 1.6s wall-clock
duration. For example, a 200ms slot time would use 8 slots per leader span.

This preserves leader-window duration but requires the leader schedule to become
dynamic across feature activations. It affects code that computes leader-window
boundaries, leader schedule offsets, next-leader routing, replay propagation,
Alpenglow parent-ready behavior, and related client assumptions. The
experimental Agave implementation that changes leader span and epoch length
shows this introduces significant code complexity and therefore higher risk of
mistakes.

This proposal keeps the leader span at 4 slots and instead steps the slot time
down gradually. That approach directly tests whether the shorter slot and
leader-window timings close without introducing a simultaneous leader schedule
change.

### Increase Slots Per Epoch

Another alternative is to increase `slots_per_epoch` as slots get faster so an
epoch remains approximately two days. At 200ms slots this would require 864,000
slots per epoch.

This preserves epoch wall-clock duration, but requires dynamic epoch schedule
handling and increases client implementation risk. Existing code commonly uses
epoch boundaries to derive leader schedules, epoch stakes, rewards, and
wall-clock estimates. Changing epoch length at the same time as slot time
increases the blast radius.

This proposal keeps epochs at 432,000 slots. Epochs therefore become shorter in
wall-clock time, reaching approximately one day at 200ms slots. That tradeoff
is acceptable for a staged rollout and gives the network faster feature
activation cadence.

### Reduce Slots Per Leader Span While Keeping 400ms Slots

A third alternative is to keep 400ms slots and reduce the leader span, for
example from 4 slots to 2 slots. This gets some of the shorter-leader-window
market-structure benefit without changing slot time.

The downside is that it does not provide finer-grained slot time for market
makers, oracle freshness checks, or applications that reason about time in
slots. It also spends implementation and operational risk on leader rotation
without moving the chain toward shorter slots. If a 2-slot leader span exposes
leader-rotation problems at 400ms slots, the network may be stuck with 400ms
slots for a while before it can safely attempt 200ms slots, where a 2-slot
leader span would imply 400ms leader rotation.

## Impact

Dapp developers and off-chain services get faster slot-level feedback and lower
latency, but should stop assuming `400ms * slots` is a stable wall-clock
conversion. SDK and RPC consumers that need wall-clock estimates should prefer
cluster-provided parameters once available.

Validators get shorter leader windows and less time to receive, replay, and
build on the previous leader's block. The proportional per-slot reductions keep
execution, account writes, allocation, and Turbine data budgets close to the
current wall-clock rate, but voting and gossip activity increase because there
are more slots per wall-clock interval.

Core contributors need to remove or audit static slot-time assumptions in
runtime, consensus, Turbine, gossip, repair, RPC, CLI, tests, and client
heuristics. Over time, these parameters should move on chain so SDK constants
do not need to encode cluster reality.

## Security Considerations

This is a consensus-breaking change and MUST be feature-gated. Validators that
do not implement the effective slot-time stage, including the one-epoch delay,
may produce or accept blocks, shreds, or rewards using the wrong limits.

Shorter slots reduce the time available for leader handoff, block propagation,
replay, and vote landing. The staged rollout is part of the safety mechanism:
each stage should be observed under production conditions before activating the
next.

The proportional reductions are required to avoid accidentally increasing
per-second execution, write, allocation, and shred budgets. Without these
reductions, shorter slots would increase validator resource requirements and
could harm liveness.

Inflation code must use wall-clock-equivalent slot accounting. Failing to scale
`slots_per_year`, or mishandling effective-transition boundaries, could change
issuance.

Validators must not enforce a reduced shred limit during a feature's one-epoch
delay period. Doing so could cause Turbine or shred-fetch filtering to drop
valid shreds before execution and block validation have switched to the new
limits.

## Drawbacks

Faster leader spans make it harder for clients and searchers to target the
right leader. Fanout can compensate, but it may reduce transaction privacy and
increase network load.

Vote costs are not changed. Validators therefore pay more vote fees per
wall-clock time as slots get faster, doubling at 200ms slots.

Gossip traffic for votes and epoch slots increases with slot rate, also
doubling at 200ms slots.

Because blockhash and status cache max entries are not changed, any user-facing
or client-facing behavior measured in number of slots may become shorter in
wall-clock time. This is accepted as part of the smaller initial change surface.

## Backwards Compatibility

This proposal is not backwards compatible for validators. New feature gates
change consensus-critical bank, block, and shred limits.

The SDK constants will be out of sync with chain reality until the SDK is
updated or the parameters are made available on chain. In particular, static
constants for default slot duration, ticks per second, slots per year, and
slot-to-time conversion may continue to describe the old 400ms assumption while
the running cluster uses 350ms, 300ms, 250ms, or 200ms banks. Some short-term
light breakage in clients that use these constants is tolerable in order to
ship the staged reduction sooner.

A longer-term compatibility solution is to expose the effective timing and limit
parameters on chain or through a stable RPC surface, allowing SDKs and clients
to query cluster reality instead of depending on compile-time constants.

## Conformance

Each validator implementation MUST include tests or fixtures that demonstrate:

- Activation of each feature gate from a 400ms baseline, including the
  one-epoch delay before the gate becomes effective.
- Correct bank values for `ns_per_slot`, `slots_per_year`,
  `rent_collector.slots_per_year`, and non-Alpenglow `hashes_per_tick`.
- Correct block, writable-account, vote, accounts-data, shred, and PER limits
  for every target slot time in the tables above.
- Correct snapshot restore behavior after each feature gate is active and after
  each gate becomes effective.
- Correct snapshot manifest serialization before activation, during the
  one-epoch delay, and after the effective transition for `ns_per_slot`,
  `hashes_per_tick`, and `slots_per_year`.
- Correct shred validation before activation, during the one-epoch delay, and
  after the effective transition.
- Correct inflation behavior for slot ranges and epochs that span effective
  transitions.
- Correct Alpenglow interaction, specifically that Alpenglow hashing behavior
  is not reduced by these feature gates.

The change should be accompanied by localnet ledgers that cross all four
feature activations and their delayed effective-transition boundaries.

## References

1. Agave exploratory implementation for halving slot times:
   https://github.com/anza-xyz/agave/pull/10740
2. SIMD discussion #469 in the SIMD repository:
   https://github.com/solana-foundation/solana-improvement-documents
3. Agave exploratory implementation for changing leader span and epoch length:
   https://github.com/anza-xyz/agave/pull/12154
