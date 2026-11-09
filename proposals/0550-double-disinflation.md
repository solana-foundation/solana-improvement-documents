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
rate uses the SIMD-0607 integer schedule, which must activate first. At the
SIMD-0550 activation slot, calculate `anchor_rate` using the preceding 15%
integer taper across the actual slot-time regimes, including the terminal-rate
clamp. Apply the 30% integer taper to that anchor from the activation slot
forward. At zero elapsed slots, the rate is exactly `anchor_rate`, so issuance
does not jump at activation. The stored `Inflation` governor keeps
`initial = 0.08` and changes `taper` to `0.30`; it is not re-anchored with
`powf`.

Regarding activation timing, features activate on an epoch boundary: the gate
fires on the E -> E+1 transition, and the 30% schedule governs rewards
for E+1 and later. Rewards for the just-completed epoch E are computed at that
boundary using the rate at the boundary slot. Because zero elapsed slots leave
the rate unchanged, epoch E settles identically to the pre-activation schedule and
nothing is applied retroactively. Implementations *must not* apply the
30% schedule to any epoch before E+1.

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

- Transaction Execution (Runtime): Epoch reward calculation uses the integer
  activation anchor and 30% taper, with no change to per-transaction execution.
- Virtual Machine: None.
- Block Packing: None.
- Consensus: Reward totals feed bank capitalization and the bank hash, so all
  clients must compute identical values across the activation boundary.
- Gossip: None.
- Turbine: None.
- Snapshots: The stored `Inflation` governor retains `initial = 0.08` and the
  updated taper; the feature activation slot must remain available after restart.
- On-Chain Core BPF Programs: None.
- Other (please describe): `getInflationGovernor` returns the stored
  `initial = 0.08` and updated `taper = 0.30`. The current floating-point
  `getInflationRate` calculation does not use the consensus integer schedule;
  making it do so requires a separate RPC code change. No RPC interface changes
  are specified here. Adds the feature `double_disinflation_rate`.

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

- Compute `anchor_rate` with the preceding SIMD-0607 15% integer schedule across
  the actual slot-time regimes, including the terminal-rate clamp.
- Set the stored `Inflation` taper to `0.30` and leave `initial = 0.08`.
- Apply the SIMD-0607 30% integer taper to `anchor_rate` from the activation
  slot forward. At zero elapsed slots, the result *must* equal `anchor_rate`.

Rewards for the just-completed epoch use the boundary rate. Conformance uses
the SIMD-0607 integer arithmetic and rounding rules, including the terminal
clamp; no floating-point `powf` calculation determines consensus rewards.

Conformance is verified by replaying a reference ledger spanning the activation
slot and confirming byte-identical per-epoch reward capitalization and bank
hashes. Agreement on the inflation rate alone is necessary, but clients must
also allocate the resulting rewards identically. The reference ledger and test
vectors would need to be produced by the future reference implementation, as
they depend on the activation epoch.
