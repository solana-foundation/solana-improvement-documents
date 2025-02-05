---
simd: 'XXXX'
title: Title of SIMD
authors:
  - Ashwin Sekar
  - Max Resnik
category: Standard
type: Core
status: Review
created: 2024-02-05
feature: VinfVbnL5Qh4fvfp1zwifZLAWfacbLgxpH6yaCdYydw
---

## Summary

This proposal adjusts the derivation that determines the inflation rewards a validator
receives as a function of their stake and vote credits. The current function incentivizes
validators to undermine the credits of their colleagues in order to earn more inflation
rewards for themselves.

Under the new system a validator will have a fixed portion of the total inflation
rewards that they are eligble to earn. They strive to earn the maximal vote credits
in order to earn as much of this fixed portion as possible, with the remaining being
burned.

## Motivation

The current rewards calculation distributes inflation rewards based on the
relative distribution of vote credits and stake across the cluster. This results
in a validator earning higher rewards if other validators produce lower vote credits.

Although not immediately obvious if such activity is happening on mainnet, this
creates an incentive for validators to engage in malicious activity such as censoring
votes, slowing down the dissemnitation of shreds, not participating in repair, etc.
These malicious activites cannot be detected rigorously and thus are unable to be
slashed.

There is still a balance that a malicious validator must strike, as hampering the
voting ability of the cluster could cause their own leader blocks to be
skipped. However this is only relevant when there are competing forks. Mainnet has
very few forks as of late which makes these strategies more viable.

Instead we propose a system that removes this incentive to harm fellow cluster
participants. By isolating the inflation rewards a validator is able to receive,
we greatly reduce the effect of another validator earning less vote credits.

## New Terminology

N/A

## Detailed Design

### Current economics

The current inflation rewards $R(v)$ that a validator receives is based on the
total cluster points $P$. Let each validator $v \in V$ have active stake $S(v)$
and vote credits $C(v)$.

$$P = \sum_{v\in V}S(v)C(v)$$

The reward is then calculated with the total inflation rewards $I$ as a fraction
of $P$:

$$R(v) = I\times\dfrac{S(v)C(v)}{P}$$

### Proposed economics

When the feature flag `vote_credit_inflation_calculation` is active we will instead:

- Calculate the maximum vote credits $M$ that are possible to be earned in an epoch
  with 0 blocks skipped.
  As of writing this proposal $M = 16 \times 432000 = 6912000$

- For each validator determine the portion of inflation rewards they are entitled
  to as a function of their stake to the total cluster stake:

  $$P(v) = I \times \dfrac{S(v)}{\sum_{u\in V}S(u)}$$

- Award rewards based on the ratio between each validators credits and $M$:

  $$R(v) = P(v) \times \dfrac{C(v)}{M}$$

- Burn any rewards not earned $B$

  $$B = \sum_{v\in V}\left(P(v) \times \dfrac{M - C(v)}{M}\right)$$

## Alternatives Considered

What alternative designs were considered and what pros/cons does this feature
have relative to them?

## Impact

// TODO(max): Explain how there is still incentive based on $B$, but that it is
// much lower now

// TODO(max): Share some numbers about how we expect the rewards distribution to
// change as well as the estimated inflation rate based on skip rate.

A future change will explore awarding the leader credits based on how many timely
votes they can pack into a block. This could further align incentives and remove
any censorship motivation that still exists based on the burned rewards.

## Security Considerations

Rewards calculation changes should be rigorously reviewed and audited as bugs
here could result in widespread economic impact.

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility

This feature is not backwards compatible.
