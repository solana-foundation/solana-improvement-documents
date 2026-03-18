---
simd: '0497'
title: Reduce Consecutive Leader Slots to 2
authors:
  - cavey
category: Standard
type: Core
status: Draft
created: 2026-03-18
feature: 9N4TN7bBtviskXWRo8pFcvp4caa6pPUQTx9BRWyeYLo
---

## Summary

Reduce `NUM_CONSECUTIVE_LEADER_SLOTS` from 4 to 2 via a single feature
gate activation.

A reference implementation exists in a private repository; a public
source link will be added when it is published.

## Motivation

The current 4-slot leader window (~1.6s) was chosen early in Solana's
design when network conditions and validator software were less mature.
Reducing the leader window improves the network on several axes:

**Reduced effects of downtime.** A malicious or delinquent leader
currently costs a user 1.6s of dead time (4 empty slots). With 2-slot
windows, a skip costs 800ms. (slightly better under Alpenglow, as a skip
allows the next leader to produce immediately without waiting for the
remainder of the prior leader's consecutive span.

**Lower variance in leader schedule distribution.** The leader schedule
RNG samples once per leader group. With 4-slot groups, an epoch of 432,000
slots produces 108,000 samples. With 2-slot groups, it produces 216,000
samples. More samples converge closer to true stake-weighted proportions,
reducing variance for smaller validators.

**Reduced MEV extraction window.** A shorter contiguous leader window
limits the time a single leader has to reorder, censor, delay, or
sandwich transactions across consecutive blocks.

## New Terminology

N/A

## Detailed Design

### Feature Gates

One feature gate SHALL be introduced:

- `reduce_consecutive_leader_slots` — when activated, the leader schedule
  repeat factor changes from 4 to 2.

### Core Change

A function replaces the compile-time constant:

```rust
pub fn num_consecutive_leader_slots(feature_set: &FeatureSet) -> u64 {
    if feature_set.is_active(&reduce_consecutive_leader_slots::id()) {
        2
    } else {
        4
    }
}
```

All call sites currently referencing `NUM_CONSECUTIVE_LEADER_SLOTS` MUST
be updated to call this function with access to the relevant `FeatureSet`.

### Leader Schedule Generation

The leader schedule assigns each validator a contiguous run of slots (the
repeat factor). Validators are sampled by stake weight once per run; that
leader then produces blocks for the next `repeat` slots. Schedules are
computed at epoch boundaries from the bank state at that boundary. The
change: `leader_schedule()` passes
`num_consecutive_leader_slots(&bank.feature_set)` as the repeat parameter
instead of the constant `NUM_CONSECUTIVE_LEADER_SLOTS`; feature
activations take effect at epoch boundaries, so all validators agree on
the repeat factor for any given epoch.

**Feature Activation.** Leader schedules are computed one epoch in advance,
so the repeat factor does not flip the instant a feature activates:

1. The feature is pending during epoch N.
2. The feature activates on the transition from epoch N to N+1.
3. The schedule for epoch N+1 is still generated with repeat factor 4;
   the first schedule generated with repeat factor 2 is for epoch N+2.

Implementations MUST NOT retroactively replace already-committed future
schedules; validators agree on the bank state at each epoch boundary used
to build the schedule for a given epoch.

### Notes

**Grace ticks.** In agave, the grace formula already uses `num_slots`
(from the leader schedule), so no code changes are required. The grace
period scales proportionally with the leader window: at 4 slots, grace
is 128 ticks (~800ms); at 2 slots, 64 ticks (~400ms). This is the primary
parameter to monitor for skip rate impact after activation. We could
introduce another change to restore the existing grace period, but there
is also a general desire to reduce the grace period, so we keep it as-is.

**Client implementations.** Client teams should audit their codebases for
any use of `NUM_CONSECUTIVE_LEADER_SLOTS` and ensure call sites use the
feature-gated value. e.g. in agave the poh recorder
(`is_same_fork_as_previous_leader`), replay stage (propagation check,
`should_retransmit`), banking stage (forward/hold decision, although we
are probably going to delete forwarding), RPC (`ClusterTpuInfo` /
`get_leader_tpus`, although RPC may be removed),
`first_of_consecutive_leader_slots`.

### Activation Sequence

The feature gate is activated via the standard feature activation
mechanism. No special coordination is required beyond the normal
feature activation process. See **When the new repeat factor applies**
under Leader Schedule Generation for the epoch relationship between
activation and the first epoch whose schedule uses repeat factor 2.

## Alternatives Considered

**Reducing slot duration instead ([Discussion #469]).** Halving slot time
(e.g. 400ms to 200ms) is a far more invasive change and extra voting overhead.

[Discussion #469]: https://github.com/solana-foundation/solana-improvement-documents/discussions/469

**Gradual step to 3 slots before 2.** A two-step rollout (4→3 then 3→2)
would add a second feature gate and more activation coordination. This
SIMD specifies a single gate from 4 to 2 for simplicity; a phased
reduction remains a possible future choice if operators want incremental
risk reduction.

**Reducing directly to 1 slot.** A future proposal could reduce the
window further from 2 to 1. The 4→2 step is low risk and provides
empirical data on the effects of shorter windows before considering a
further reduction. A change to 2 slots is also on par with the halve slot
duration proposal, as it halves the leader window duration.

## Impact

**App Developers & Landing Services.** Leader targeting is a little harder
with 800ms leader windows, but I have confidence the market will figure
it out.

Many clients and landing services today assume a fixed consecutive-leader
span (often 4) in constants or heuristics. Across activation, they MUST
stop relying on a hard-coded span: read the span from chain state (via
the feature-gated schedule and APIs that expose consecutive leader
length) or update deployed configuration at the activation boundary.
This SIMD does not introduce an on-chain account solely to publish the
span; operators are expected to ship software that uses the schedule or
feature set rather than a stale constant.

**Block Explorers / Analytics.** Tools that aggregate metrics per "leader
rotation" (grouping 4 consecutive slots) need updating to group 2
consecutive slots (e.g. firedancer gui).

## Security Considerations

**Skip rate.** With a shorter leader window, a validator that is slow to
receive the parent block has less time before its slot expires if we keep
the current grace period formula. We need to run stability experiments.
Testnet should be good enough imo. More frequent leader rotations and
possibly higher skip rate mean more opportunities for forks at leader
boundaries, so we may need to change the towerBFT threshold parameters.

**PoW takeover (pre-Alpenglow).** Before Alpenglow, a shorter consecutive
leader span means fewer PoH hashes within a leader's window to reach the
next leader's slot, which can make PoW-style takeover by the next leader
relatively more viable than with a 4-slot span. This should be weighed
alongside Alpenglow deployment, which changes leader progression after
skips. This is true of anything that reduces hashes/window, such as
reducing slot times.

## Backwards Compatibility

This is a consensus-breaking change gated behind a feature activation.
Nodes running software that does not support this feature gate will
diverge from the cluster upon activation. All validator client
implementations (Agave, Firedancer, Sig, etc.) MUST implement support
for this feature gate before it is activated on mainnet-beta.

The leader schedule for epochs prior to activation is unaffected.
Historical queries over old epochs return the same schedule as before.
