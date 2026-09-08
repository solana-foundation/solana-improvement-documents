---
simd: '0630'
title: FLH Slot Time Compensation
authors:
  - Quentin Kniep
  - Roger Wattenhofer
category: Standard
type: Core
status: Review
created: 2026-09-08
feature: (fill in with feature key and github tracking issues once accepted)
extends: SIMD-0326, SIMD-0337
---

## Summary

With Alpenglow's fast leader handover, the leader starts disseminating slices
for the first slot of their leader window as soon as they have received and
executed the last slice of the previous block. However, its block production
timer of duration `Δblock` only starts once the `ParentReady` event is observed.

This proposal reduces the post-`ParentReady` production budget for the first
slot of a leader window by a constant `HANDOVER_COMPENSATION` (60 ms) from 200
ms (after full activation of SIMD-0525) to 140 ms.
Likewise, the timeout duration after `ParentReady` for the first slot is reduced
by `HANDOVER_COMPENSATION` and timeouts for other slots shift accordingly.
As we show below, with `HANDOVER_COMPENSATION` the *effective slot time* roughly
averages `Δblock` also for the first slot.

Further, in order to reduce idle time, especially in the long tail of long
handovers, each leader is now supposed to send their block directly to the
following leader, in addition to distributing it through Turbine/Rotor.

## Motivation

Fast leader handover (SIMD-0326, SIMD-0337, Section 2.7 and Algorithm 3 of the
Alpenglow white paper) aims to reduce the idle period between two consecutive
leaders.
It does so by letting the next leader start producing slices immediately upon
receiving the last block of the previous leader.
However, the additional time is added on top of the first slot’s `Δblock`, so
the first slot will typically be longer.

Consequences of leaving this uncompensated:

1. **The effective slot time would not match the targets of SIMD-0525.** The
   final stage of SIMD-0525 targets 200 ms; but with the uncompensated fast
   leader handover, a correct leader would produce for an average of around 260
   ms. This makes the first slot per window systematically longer.
2. **Epoch wall-clock duration.** As epochs are a fixed number of slots, the
   more wall-clock time elapses on top of the pure slot times, the longer the
   epoch actually takes. This impacts inflation and VAT, as both are paid per
   epoch.

Reducing the first block's time budget by `HANDOVER_COMPENSATION` improves both
problems and makes the shred-production span more uniform across slots.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0326](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0326-alpenglow.md):
  Alpenglow** — Defines `ParentReady`, leader windows, `Δblock`, and the block
  creation procedure (Algorithm 3 of the white paper) and timeouts that this
  proposal modifies.

- **[SIMD-0337](https://github.com/solana-foundation/solana-improvement-documents/pull/337):
  Markers for Alpenglow Fast Leader Handover** — Defines the `BlockHeader` and
  `UpdateParent` markers that make optimistic block production observable to the
  rest of the network and allow switching parents in the sad case.

- **[SIMD-0525](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0525-reduce-slot-times.md):
  Shorter slot times** — All numbers in this proposal assume that 200 ms slot
  times are active.

Related proposals, not dependencies:

- **[SIMD-0363](https://github.com/solana-foundation/solana-improvement-documents/pull/363):
  Simple Alpenglow Clock** — block timestamps become a direct measurement of the
  effective slot time, and therefore the primary way to observe conformance.
- **[SIMD-0497](https://github.com/solana-foundation/solana-improvement-documents/pull/498):
  Reduce Consecutive Leader Slots to 2** — halving the window doubles the
  per-slot cost of an uncompensated handover.

## New Terminology

- **`HANDOVER_COMPENSATION`**: a protocol constant, 60 ms, subtracted from the
  post-`ParentReady` production budget of the first block of a leader window and
  the timeouts.
- **Effective slot time**: the time interval between the moment a leader starts
  producing a block and the moment it sends out the last shred. For every slot
  other than the first of a window this is already supposed to equal `Δblock`.
  Another validator can only approximately observe compliance with this time
  interval, but cannot verify it.
- **Optimistic prefix**: for the first slot of a leader window, the wall-clock
  interval between the start of block production and the moment `ParentReady(s,
  ·)` enters `state[s]`.

## Detailed Design

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC
2119](https://www.ietf.org/rfc/rfc2119.txt) and [RFC
8174](https://www.ietf.org/rfc/rfc8174.txt).

### Constants

```
HANDOVER_COMPENSATION  = 60 ms
```

`HANDOVER_COMPENSATION` is an absolute duration and MUST NOT be scaled with the
slot-time stage of SIMD-0525 or any future changes to the slot time or number of
slots per leader window:
It approximates a network and vote verification and aggregation latency, which
does not shrink when the slot time shrinks.

### Modified block creation

Algorithm 3 of the Alpenglow white paper is amended as follows.
Only line 11 changes from `while clock() < start + Δblock do` to `while clock()
< start + Δblock − HANDOVER_COMPENSATION do`.
We assume `Δblock > HANDOVER_COMPENSATION`.

Slots other than the first of a leader window are unaffected: they continue to
be produced over exactly `Δblock`.

### Modified timeouts

The timeouts starting from `ParentReady` for all slots of a leader window are
reduced by `HANDOVER_COMPENSATION`.
This is equivalent to calculating them with `Δblock - HANDOVER_COMPENSATION` as
slot time for the first slot and still using the full `Δblock` for all other
slots.

### Direct leader-to-leader delivery

A block producing leader should send all shreds not just to Turbine/Rotor but
also directly to the next leader.

This change widens the gap between the next leader receiving the block and
seeing the `ParentReady` event.
This is not reflected in the timeouts but allows us to increase
`HANDOVER_COMPENSATION` and turn idle time into block production time.

### Choice of 60 ms

`HANDOVER_COMPENSATION` should roughly equal the mean optimistic prefix time
observed on mainnet.
The value of 60 ms this proposal adopts was derived from a simulation based on
real-world network latencies and the current mainnet distribution.
This value already accounts for the direct leader-to-leader delivery.
It SHOULD be validated against measurements before activation.

## Alternatives Considered

1. **Floor the block production budget at `Δblock`.** Equivalently, end the
   first block at `max(t_start + Δblock, t_ready + Δblock −
   HANDOVER_COMPENSATION)`.
However, then the mean effective slot time is again strictly above `Δblock`.
Also, correct leaders might again produce blocks until `t_ready + Δblock` (in
the case where `t_start = t_ready`) and reducing the timeouts or having leaders
stop at `t_ready + Δblock − HANDOVER_COMPENSATION` would not be feasible.

2. **Start the block production timer at the optimistic start instead of at
   `ParentReady`.** The effective slot time is exactly `Δblock` in all cases,
   with no constant to tune and no distributional assumption.
Again, this would not allow us to reduce the timeouts and making leaders stop
production before `t_ready + Δblock` would make the timeouts no longer tight.

3. **Apply compensation only if the parent block doesn’t change.** This would
   give a leader that is switching parents a longer timeout, and would thus make
   falsely indicating a parent switch an economically attractive strategy that
   goes against the motivation of this proposal.

4. **Amortize the compensation across the window** (subtract
   `HANDOVER_COMPENSATION / w` from each of the `w` slots).
This also keeps the window closer to `w · Δblock` but leaves the first block
longer than the others, so it does not address the problem of non-uniform block
sizes at all.

5. **Use first block only for fast leader handover** (equivalent to this
   proposal with `HANDOVER_COMPENSATION = Δblock`).
This minimizes the maximum amount by which the first slot may exceed `Δblock`
with its effective slot time, but it makes the effective slot time of the first
slot much shorter than `Δblock` on average.

6. **Do nothing** (equivalent to this proposal with `HANDOVER_COMPENSATION =
   0`).
Simplest, and the overhead is bounded and predictable.
It would mean that the first block of each leader window is (almost always)
longer than `Δblock`, and the relative gap widens with every decrease in
`Δblock`.

## Impact

- **Users and dapp developers.** At the final SIMD-0525 stage the chain advances
  at roughly 213 ms per slot rather than roughly 229 ms, about 7.5% more slots
  per unit of wall-clock time.
Blockhash expiry, which is counted in slots, shortens correspondingly in
wall-clock terms.
Block timestamps become more evenly spaced.

- **Validators.** Leaders lose 60 ms of post-`ParentReady` packing time on the
  first block of each window, which should make up for the (on average) same
  time gained through the fast leader handover.
Per-block limits are unchanged, so the first block of a window now has both the
same limits and the same wall-clock budget as its successors.
Clients that only begin packing once the parent is settled will see reduced fill
on that block and SHOULD move packing to the optimistic start.

- **Core contributors.** Epoch wall-clock duration, inflation delivered per
  wall-clock year, and realised VAT cost per day all move roughly 55% of the way
  towards the values intended by SIMD-0525 and SIMD-0357. At `w · Δblock = 800
  ms`, the epoch falls from about 27.5 h to about 25.6 h against an intended 24
  h. The remainder is the time for the next leader to receive the block from the
  previous leader, amortised over the window, and closing it requires reducing
  that time even more, e.g. by adjusting the leader schedule, or increasing the
  length of each leader window.

## Security Considerations

- **Adherence to the block production time budget is not strictly verifiable.**
  This is not a new class of behaviour: Adherence to `Δblock` itself has never
  been verifiable.
- **Withholding optimistic production gains nothing.** This is the same as
  delaying sending out shreds for any other slot. It especially does not move
  the timeout.
- **Skip safety.** The change reduces the block production timeout and skip
  timeouts by the same amount, so it cannot increase the probability of a skip
  vote against a correct block producer.

## Drawbacks

The compensation is a tuned constant standing in for the mean of an empirical
distribution.
The real distribution depends on the protocol’s block dissemination mechanism,
the geographic stake distribution and the network conditions.
If `HANDOVER_COMPENSATION` is below the mean of that distribution, blocks will
again be longer than `Δblock` on average, and shorter if it’s above the mean.

## Backwards Compatibility

Block validity is unaffected: blocks produced under either rule are accepted by
upgraded and non-upgraded validators alike, and no wire format, marker, or
certificate changes.
Ledger replay of historical blocks is unaffected.

A feature gate is REQUIRED.
If the block production budget and the skip timeouts are reduced by the same
amount, correct leaders are no more likely to be skipped than before.
In a mixed population for the timeout rule, a leader on the old block production
rule produces its first block over the full `Δblock` while a voter on the new
timeout rule times out `HANDOVER_COMPENSATION` earlier, and votes to skip a
block that is on schedule.
Stake-weighted disagreement of this kind does not threaten safety, since quorum
intersection is unchanged, but it can prevent either a notarization or a skip
certificate from forming in the first voting round and push the slot onto the
fallback path.
While only the timeout rule requires agreement, it is RECOMMENDED that both
changes are rolled out at the same time.

The gate SHOULD be activated at least one epoch after the final SIMD-0525 stage.

## Forward Compatibility

The current design assumes `Δblock > HANDOVER_COMPENSATION` throughout.
If these values cross in the future, this needs to be considered.

The effective slot time of the first slot of each leader window SHOULD be
monitored over time, especially if changes to the block dissemination protocol,
Votor, or direct leader-to-leader delivery are made, that might influence
latency of blocks or certificates.
The choice of `HANDOVER_COMPENSATION` SHOULD be adapted if that duration moves
away significantly from `Δblock`.
