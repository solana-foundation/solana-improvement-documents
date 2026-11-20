---
simd: '0698'
title: Raise Block Limits to 100M CUs in Stages
authors:
  - Brennan Watt
category: Standard
type: Core
status: Draft
created: 2026-10-19
feature:
  - 60M - TBD
  - 70M - TBD
  - 80M - TBD
  - 90M - TBD
  - 100M - TBD
extends: SIMD-0525
---

## Summary

Increase the block compute unit (CU) limit from 50M to 100M CUs at 200ms slot
times through five feature gates: 60M, 70M, 80M, 90M, and 100M CUs.

The writable-account CU limit remains unchanged (20M CUs per block in the
assumed baseline). Shred limits and all other resource limits are unchanged by
these gates.

## Motivation

Block limits restrict the amount of work a leader can include in a block so
that validators can keep up with the network.

The available headroom is primarily in parallel execution. Increasing the
writable-account limit would allow more serial execution work that must still
fit within the shorter slot duration, so the per account limit will be held
constant. Network bandwidth headroom is also less certain, so increasing the
shred budget alongside the compute budget would introduce an additional
constraint to evaluate.

Five separate gates allows for each increase to be observed before proceeding
to the next stage.

## Dependencies

This proposal assumes the 200ms configuration described in [SIMD-0525], with
the earlier 100M block-limit feature from [SIMD-0286] active. SIMD-0525 scales
that configuration to 50M block CUs and 20M writable-account CUs at 200ms.
The 200ms stage MUST be effective before activating any gate in this proposal
(it is activated on all clusters at the time of this proposal).

## New Terminology

No new terminology is introduced.

## Detailed Design

### Block Limit Stages

| Feature gate | Max Block Units |
|--------------|-----------------|
| None of these gates active | 50,000,000 (existing baseline) |
| `raise_block_limit_to_60m` | 60,000,000 |
| `raise_block_limit_to_70m` | 70,000,000 |
| `raise_block_limit_to_80m` | 80,000,000 |
| `raise_block_limit_to_90m` | 90,000,000 |
| `raise_block_limit_to_100m` | 100,000,000 |

Intention is to activate feature gates in ascending CU order, but if multiple
gates are active, implementations MUST select the largest limit among them.

Table values are absolute CU limits per 200ms block. Implementations MUST apply
them after deriving the existing slot-time-scaled limits from SIMD-0525, and
MUST NOT scale these values again.

Leaders packing blocks and validators enforcing block cost limits MUST use the
same limit selected from the bank's active feature set.

Each gate uses the existing feature activation mechanism. No additional
one-epoch effectiveness delay is introduced.

### Other Limits

Only Max Block Units changes under these gates:

| Limit | Effect of each gate |
|-------|---------------------|
| Max Writable Account Units | Unchanged at 20M CUs |
| Max Vote Units | Unchanged |
| Max Block Accounts Data Size Delta | Unchanged |
| Max data shreds per slot | Unchanged |
| Max coding shreds per slot | Unchanged |
| Per-transaction resource limits | Unchanged |

Implementations MUST preserve the limits selected by other active features for
these resources. In particular, the writable-account limit MUST NOT be
recomputed as a proportion of the increased block limit.

## Alternatives Considered

- Keep the 50M limit. This leaves additional parallel execution capacity unused.
- Increase directly to 100M CUs with one gate. This reaches the same final
  capacity sooner, but provides no intermediate production observations.
- Increase writable-account limits alongside the block limit. This increases
  serial execution work despite the tighter execution window at 200ms slots.
- Increase shred limits alongside the block limit. This also raises the maximum
  network data budget before bandwidth headroom has been established.

## Impact

Applications whose transactions access independent writable accounts gain
additional block capacity. Workloads constrained by the per-account limit or
other unchanged limits may see no capacity increase.

Validators may execute more work per slot and per second. At the final stage,
the allowed block compute budget doubles relative to the 50M baseline, while
the target slot time remains 200ms.

## Security Considerations

Larger compute budgets can increase replay time, slow catchup, and cause
validators to fall behind. Keeping the writable-account limit fixed bounds work
on a single writable account, but does not guarantee that every allowed workload
will execute within the target slot time.

Higher sustained transaction volume can also increase resource requirements for
RPC nodes, indexers, and other infrastructure.

Unchanged shred limits bound the maximum shreds per slot. Actual bandwidth
usage can still rise as blocks become fuller. Each stage should be evaluated
under sustained load before proceeding to the next.

Feature selection and the order of limit derivation are consensus critical.
Applying slot-time scaling twice, or selecting different active stages during
production and replay, can cause validators to disagree on block validity.

## Backwards Compatibility

Previously valid blocks remain within the increased block compute limit.
However, blocks produced after activation may exceed the limit supported by
older validators and be rejected by them. Validators must support each stage
before its activation.

## Conformance

Each validator implementation MUST demonstrate:

- The existing 50M limit is retained in the assumed baseline when no new gate
  is active.
- Each gate selects its exact table value, including when earlier gates are
  absent, and the largest active stage wins when several gates are active.
- Block cost enforcement permits an accumulated cost equal to the selected
  limit and rejects a cost exceeding it, with other limits satisfied.
- The writable-account limit and all other resource limits are unchanged at
  every stage and remain enforced independently of the block limit.
- The new values are applied after SIMD-0525's slot-time scaling, with 100M CUs
  permitted at the final stage under 200ms slots.
- Block production, replay across activation boundaries, and snapshot restore
  select identical limits for the same bank.

The change SHOULD be accompanied by a localnet ledger demonstrating the
baseline and all five feature activations.

[SIMD-0286]: 0286-raise-block-limits-to-100M.md
[SIMD-0525]: 0525-reduce-slot-times.md
