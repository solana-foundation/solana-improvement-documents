---
simd: '0550'
title: Double Disinflation Rate
authors:
  - Lostin & 0xIchigo (Helius)
category: Standard
type: Core
status: Review
created: 2026-06-02
feature: (fill in with feature key and GitHub tracking issues once accepted)
---

## Summary

Reduce the inflation schedule by increasing the disinflation rate from the
current -15% rate to -30%.

## Motivation

While there is a significant appetite to reduce the nominal inflation rate of
SOL, mechanism design has become a point of contention, ultimately leading to
SIMD 228 failing to reach quorum. This SIMD represents a simplification of the
idea, delivering predictable inflation reduction by doubling the disinflation
rate.

The same change was previously proposed as [SIMD-0411](https://github.com/solana-foundation/solana-improvement-documents/pull/411);
SIMD-0550 brings it back now that governance tooling is ready and tokenomics
is, again, an ecosystem focus.

## New Terminology

N/A

## Detailed Design

Add a new feature gate called `double_disinflation_rate`. The inflation
rate is computed from the elapsed time since genesis, so simply setting
`taper = 0.30` would reshape the whole curve and drop the rate discontinuously
at activation. To double the rate of decline from activation onward while
keeping issuance continuous, the schedule is re-anchored at the activation slot:
the rate the current (`taper = 0.15`) schedule yields at the activation `year`
is recorded as `anchor_rate`, `taper` is set to `0.30`, and `initial` is
recomputed so the steeper curve passes through `anchor_rate` at that `year`
(`initial = anchor_rate / (1 - 0.30)^year`). This leaves `total(year)` unchanged
at the activation slot, so the doubled disinflation takes effect going forward
rather than as a step. The exact ordered computation and its determinism
requirements are specified under Conformance; the reference implementation is
tracked in the `feature` field.

Regarding activation timing, features activate on an epoch boundary: the gate
fires on the E -> E+1 transition, and the re-anchored schedule governs rewards
for E+1 and later. Rewards for the just-completed epoch E are computed at that
boundary using the rate at the boundary slot. Because the re-anchor leaves that
rate unchanged, epoch E settles identically to the pre-activation schedule and
nothing is applied retroactively. Implementations *must not* apply the
re-anchored schedule to any epoch before E+1.

The feature gate is retained permanently and not cleaned up. Inflation rewards
are committed to bank hashes, so the effective taper must be a function of
slot so that any node reconstructing the chain from genesis arrives at the
same state. Removing the gate would cause a from-genesis replay that crosses
the activation slot to recompute pre-activation epochs with the wrong taper.

This proposal does not modify `DEFAULT_TAPER` in `solana-inflation`. That
constant is only consulted when a new genesis is constructed; it has no
effect on already-running clusters, whose taper is carried in bank state
and updated by the gate above.

### Validator Components Affected

- Transaction Execution (Runtime): Epoch reward calculation only;
  `taper`/`initial` are re-anchored at the activation slot, with no change to
  per-transaction execution.
- Virtual Machine: None.
- Block Packing: None.
- Consensus: Reward totals feed bank capitalization and the bank hash, so all
  clients must compute identical values across the activation boundary.
- Gossip: None.
- Turbine: None.
- Snapshots: `Inflation` must persist in the serialized bank fields so the
  re-anchored values survive a restart.
- On-Chain Core BPF Programs: None.
- Other (please describe): RPC output (`getInflationRate`,
  `getInflationGovernor`) reflects the new schedule; `getInflationGovernor`
  returns the re-anchored `initial`. There is no API change. Adds the feature
  `double_disinflation_rate`.

## Alternatives Considered

- Do nothing, leave inflation as is.
- Invent a new inflation mechanism, previously deemed contentious and
  unpalatable.
- Directly halve the inflation rate, resulting in a faster reduction to
  inflation at the expense of a sudden, drastic drop in validator profitability.

## Impact

Doubling disinflation accelerates the timeline of reaching the terminal
emissions rate from a period of ~5.7 years to ~2.8 years. This would result in
a reduction of approximately ~18.9 million SOL in emissions over the next 6
years, which is ~2.6% lower than the current disinflation schedule. With 41% of
validators already opting for a 0% commission on emissions, this change would
result in little realized reduction in revenue for many validators, with a soft
taper so the remaining 59% do not experience any immediate significant shock to
projected earnings. A more detailed breakdown can be found in the [accompanying
forum post](https://forum.solana.com/t/simd-0550-proposal-to-double-disinflation/4874).

## Security Considerations

The taper transition is consensus-affecting because rewards are committed to
bank hashes. The `double_disinflation_rate` feature gate, therefore, encodes
a slot-dependent value (i.e., `0.15` before activation, and `0.30` at and
after it) and must remain in the client for as long as any node may replay
history across the activation slot; in practice, permanently.

## Backwards Compatibility

This change is not backwards compatible and requires a feature gate activation.

## Conformance

This change is consensus-affecting: epoch inflation rewards feed into bank
capitalization and, therefore, the bank hash. Every client implementation
*must* compute identical reward values across the activation boundary or risk
diverging from the canonical chain.

At the activation slot, clients *must* perform the following steps in order:

- Compute `year` from the slot exactly as the reward path does
- Compute `anchor_rate = total(year)` under `taper = 0.15`
- Set `taper = 0.30`
- Set `initial = anchor_rate / (1.0 - taper).powf(year)`

All arithmetic uses IEEE-754 binary64, meaning clients must use the same
decimal literals (i.e., `0.08`, `0.015`, `0.15`, and `0.30`), each taken to its
nearest `f64`. Because `foundation = 0.0`, the rate applied to rewards is
`validator(year) == total(year)`.

For all subsequent slots, clients use the re-anchored `initial` in the
following rate calculation:

```text
total(year) = max(0.015, initial * (1 - 0.30)^year)
```

Conformance requires bit-for-bit agreement on each `powf` evaluation.

Conformance is verified by replaying a reference ledger spanning the activation
slot and confirming byte-identical per-epoch reward capitalization and bank
hashes. Agreement on the inflation rate alone is necessary, but clients must
also allocate the resulting rewards identically. The reference ledger and test
vectors would need to be produced by the future reference implementation, as
they depend on the activation epoch.
