---
simd: 'XXXX'
title: Title of SIMD
authors:
  - Ashwin Sekar
  - Max Resnick
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

In addition, the new system will introduce credits for timely vote inclusion. These timely inclusion 
credits will be awarded to validators that include timely votes in their blocks, offsetting the incentive to artificially delay inclusion votes from other validators.

## Motivation

The current rewards calculation distributes inflation rewards based on the
relative distribution of vote credits and stake across the cluster. This results
in a validator earning higher rewards if other validators produce lower vote credits.

Although not immediately obvious if such activity is happening on mainnet, this
creates an incentive for validators to engage in malicious activity such as censoring
votes, slowing down the dissemnitation of shreds, not participating in repair, etc.
These malicious activites are not part of the forensic support (one cannot prove that they have occurred) therefore they cannot be slashed.

The goal of this proposal is to remove the incentive for validators to engage in these malicious activities by changingthe way timely vote credits are applied introducing a new issuance based reward for timely vote inclusion.

## New Terminology
1. $V$ is the set of all validators
2. $v \in V$ is a validator
3. $s_v \in [0,1]$ is the stake fraction of validator $v$
4. $c_v \in [0,1]$ is the vote credits of validator $v$
5. $S = \sum\limits_{v \in V} s_v$ is the total cluster stake
6. $C = \sum\limits_{v \in V} c_v$ is the total cluster vote credits
7. $I$ is the total inflation rewards issued in an epoch
8. $\text{VIC}_{i,j}$ are the proposed vote inclusion credits per credit earned by validator $j$

## Detailed Design

### Current economics

This formula incentivizes validators to undermine the vote credits of their colleagues in order to earn more rewards for themselves. Each individual validator's rewards are given by:

$$
R_i(s_1,\dots,s_n,c_1,\dots,c_n) = I \frac{s_ic_i}{\sum\limits_{v \in V} s_v c_v}
$$

If a validator $i$ has stake fraction $s_i$ and vote credits $c_i$ then the cost to him of a marginal increase in vote credits for a validator $j$ with stake fraction $s_j$ and vote credits $c_j$ is:

$$
\frac{\partial R_i}{\partial c_j} = I \frac{s_ic_is_j}{\left(\sum\limits_{v \in V} s_v c_v\right)^2}
$$

This tells us that if we want incentive alignment we need to give the leader at least  $\frac{\partial R_i}{\partial c_j}$ in rewards for every vote credit validator $j$ gains. But if we use this formula exactly, we would end up with validator rewards proportional to squared stake. This is because inclusion rewards earned would be proportional to the number of leader slots a validator has which is proportional to their stake and there is a $s_i$ term in the numerator of the formula. This is not desirable as it would incentivize validators to accumulate more stake or merge with other small validators. To mitigate this we propose replacing the $s_i$ term in the numerator with $.33$ thereby insuring that inclusion rewards are not proportional to stake but are still high enough to incentivise inclusion for any validator with less than $f/3f+1$ of the stake.

$$
\text{VIC}_{i,j} = I \frac{.33s_j}{\left(\sum\limits_{v \in V} s_v c_v\right)^2}
$$

with this formula, leader rewards for a slot $k$ are proportional to

$$
\text{LeaderRewards}_k = \sum\limits_{j \in V} \text{VIC}_{i,j} = I \frac{.33}{\left(\sum\limits_{v \in V} s_v c_v\right)^2} \sum\limits_{j \in V} s_j
$$

If everyone is performing well i.e. $c_v \approx 1, \forall v \in v$ then this comes out to vote inclusion rewards per block being equal to 1/3 of vote credi rewards per block. So we should split our total inflation budget into 4 quarters and allocate 1 quarter to vote inclusion credits and the other 3 quarters to timely vote credits.


### Proposed economics

When the feature flag `vote_credit_inflation_calculation` is active we will instead: 

- Calculate the maximum vote credits $M$ that are possible to be earned in an epoch
  with 0 blocks skipped.
  As of writing this proposal $M = 16 \times 432000 = 6912000$

- For each validator determine the portion of inflation rewards they are entitled
  to as a function of their stake to the total cluster stake:

  $$P(v) = I \times \dfrac{S(v)}{\sum\limits_{u\in V}S(u)}$$

- Award rewards based on the ratio between each validators credits and $M$:

  $$R(v) = P(v) \times \dfrac{C(v)}{M}$$

- Burn any rewards not earned $B$

  $$B = \sum\limits_{v\in V}\left(P(v) \times \dfrac{M - C(v)}{M}\right)$$

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
