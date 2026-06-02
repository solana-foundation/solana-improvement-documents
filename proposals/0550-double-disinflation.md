---
simd: '0550'
title: Double Disinflation Rate
authors:
  - Lostin & 0xIchigo (Helius)
category: Standard
type: Core
status: Review
created: 2026-06-02
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Reduce the inflation schedule by increasing the disinflation rate from the
current -15% rate to -30%.

## Motivation

While there is a significant appetite to reduce the nominal inflation rate of
SOL, mechanism design has become a point of contention, ultimately leading to
SIMD 228 failing to reach quorum. This SIMD represents a simplification of the
idea, delivering predictable inflation reduction by simply doubling the
disinflation rate.

## New Terminology

N/A

## Detailed Design

Add a new feature gate to Agave called `double_disinflation_rate`. Upon feature
gate activation, in the `Bank`, execute this function to double the current
taper of `0.15` to the new value of `0.30`.

```rust
if new_feature_activations
    .contains(&feature_set::double_disinflation_rate::id())
{
    let mut inflation = self.inflation.write().unwrap();
    inflation.taper = 0.30;
}
```

The feature gate is retained permanently and not cleaned up. Inflation rewards
are committed to bank hashes, so the effective taper must be a function of
slot so that any node reconstructing the chain from genesis arrives at the
same state. Removing the gate would cause a from-genesis replay that crosses
the activation slot to recompute pre-activation epochs with the wrong taper.

This proposal does not modify `DEFAULT_TAPER` in `solana-inflation`. That 
constant is only consulted when a new genesis is constructed; it has no
effect on already-running clusters, whose taper is carried in bank state
and updated by the gate above.

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
projected earnings. A more detailed breakdown can be found in the accompanying
forum post.

## Security Considerations

The taper transition is consensus-affecting because rewards are committed to
bank hashes. The `double_disinflation_rate` feature gate, therefore, encodes
a slot-dependent value (i.e., `0.15` before activation, and `0.30` at and
after it) and must remain in the client for as long as any node may replay
history across the activation slot; in practice, permanently.

## Backwards Compatibility

This change is not backwards compatible and requires a feature gate activation.
