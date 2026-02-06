---
simd: '0326'
title: Alpenglow
authors:
  - Quentin Kniep
  - Kobi Sliwinski
  - Roger Wattenhofer
category: Standard
type: Core
status: Review
created: 2025-07-25
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This proposal changes the core consensus protocol from the current one, based on
**Proof-of-History** and **TowerBFT**, to **Alpenglow**, specifically the
**Votor** parts of Alpenglow. Compared to the old protocol, **Alpenglow** offers
higher resilience and better performance.

This SIMD comes with an extensive companion paper. The Alpenglow White Paper
v1.1 is available at https://www.anza.xyz/alpenglow-1-1.

More precisely, this SIMD covers everything in the v1.1 White Paper **except**

- Section 2.2 Rotor: Initially we stay with Turbine as the data dissemination
  protocol. Rotor will be introduced later and will get its own SIMD. 
- Section 3.1 Smart Sampling: Related to Rotor, will be included in the Rotor
  SIMD. 
- Section 3.2 Lazy (Asynchronous) Execution: Has its own SIMD.


## Motivation

The current TowerBFT consensus protocol: 

- has a consensus finality time of 12.8 seconds, 
- additionally provides earlier pre-confirmations, 
- and does not have a security proof, which is concerning.

Alpenglow takes into account the current state of the art in consensus research
to improve the consensus protocol in all points mentioned above. Alpenglow:

- Lowers actual consensus finality latency below the pre-confirmation latency of
  TowerBFT. 
- Decreases bandwidth use, e.g., by eliminating costly gossip traffic.
- Reduces the consensus computation overhead, e.g., by replacing on-chain
  signature verification with local signature aggregation. 
- Increases resilience on all fronts (sophisticated attackers, adversarial
  network conditions, DOS attacks and crash failures). 
- Removes harmful incentives, such as waiting to cast more profitable votes.


## New Terminology

**Alpenglow** is a set of core protocol changes proposed in the new white paper
and related subsets of changes: Votor, Rotor, Blokstor, Pool.

**Slice** is the data structure between block and shred, i.e., a block is sliced
into slices, which are then shredded into shreds. A slice was formerly called
FEC-set.

**Timeouts** are short periods of time measured locally by the nodes. Their
purpose is akin to Proof-of-History, but without any notion of synchronized
time. The absolute wall-clock time and clock skew have no significance to the
protocol. Even extreme clock drift can be simply incorporated into the timeouts,
e.g. to tolerate clock drift of 5%, the timeouts can simply be extended by 5%.
As explained later, Alpenglow is partially-synchronous, so no timing- or
clock-related errors can derail the protocol.

**Vote** is an existing term already, but votes are different in Alpenglow. In
Alpenglow, votes are not transactions on chain anymore, but just sent directly
between validators. Also, votes do not include lockouts.

**Certificate** is a proof that a certain fraction of nodes (by stake) cast a
specific type of vote. This can be efficiently implemented as an aggregate (BLS)
signature corresponding to the set of individual vote signatures.

**Votor** is a set of protocols related to voting, block production and
finalization.

**Pool** is the data structure managing observed votes and certificates,
informing Votor.

**Fast-finalization** is the mechanism of finalizing a block after observing one
round of votes for the block from 80% of the stake.

**Slow-finalization** is the mechanism of finalizing a block after observing two
rounds of votes for the block from 60% of the stake.

**Validator Admission Ticket** (VAT) is a mechanism translating the current cost
of voting into a similar economic equilibrium for Alpenglow.


## Detailed Design

Here we only give a short rundown of the parts that will be most visible to
validator operators. For more details and proofs please read the Alpenglow white
paper.


### Voting

Voting proceeds in two rounds: 

- In the first round, validators vote either to *notarize* a specific block or
*skip* the slot, based on whether they saw a valid block before their local
timeout. 
- In the second round, validators vote *finalize* if they saw enough *notarize*
votes in the first round. Otherwise there are two conditions (*safe-to-notar*
and *safe-to-skip*, explained in the white paper) that cause the validators to
vote *notarize-fallback* or *skip-fallback*. 

Votes are distributed by broadcasting them directly to all other validators.


### Certificates

There are five types of certificates: 

- Notarization: corresponds to 60% of *notarize* votes. 
- Skip: corresponds to 60% of *skip* or *skip-fallback* votes.
- Finalization: corresponds to 60% of *finalize* votes. 
- Fast-Finalization: corresponds to 80% of *notarize* votes. 
- Notar-fallback: corresponds to 60% of *notarize* or *notar-fallback* votes.


### Finality

A slot can be directly finalized (and thus decided) in one of two ways: 

- Create or receive a *fast-finalization* certificate. 
- Create or receive a *slow-finalization* certificate and a *notarization*
certificate. 

Whenever a
block *b* in slot *s* is finalized directly, all previous slots that were
undecided are decided indirectly. All ancestors of *b* are finalized, and slots
omitted in the chain of *b* are skipped.

Liveness (proved in the white paper) ensures that eventually a block from an
honest leader will be finalized, thus finalizing all ancestor blocks. The white
paper also proves that safety holds, i.e., if a block b is finalized, then all
future finalized blocks are descendants of b.


### Rewards

In this SIMD we focus on the consensus-related benefits of Alpenglow. Below, we
translate the existing vote rewards as they are (same mechanisms, just different
programs), while removing some harmful incentives (such as the incentive to wait
before casting a vote). Economic changes are left to future economics-focused
SIMDs.

In this proposal we make sure that nodes that do not participate in the protocol
will not be rewarded. Towards this end, all nodes prove that they are voting
actively. In slot *s*+8 (and only in that slot), the corresponding leader can
post up to two vote aggregates (a notarization aggregate and/or skip aggregate)
for all votes it has seen for slot *s*. Aggregates are like certificates but
without a requirement to meet any minimum certificate thresholds. Alpenglow
ensures that each validator casts exactly one of the two relevant votes, while
casting both votes is a provable offence.

The rewards are computed as follows:

- Epoch inflation: E = (1+(yearly inflation))^((epoch length) / year)-1 
- Inflation in SOL per slot: T = (total SOL supply) * E / (slots per epoch) 
- Given validator’s stake: R = (validator’s stake) / (total stake)

For each submitted vote in the two aggregates, the voter receives *R*x*T*/2 SOL,
where *R* is the voter’s fractional stake and *T* is the total target issuance
per slot. The submitter (leader) gets the same amount of SOL as each of the
voters included in the aggregate. Nodes receiving 0 SOL at the end of the epoch
are removed from the active set of nodes. This scheme will practically eliminate
today’s voting transaction overhead while still rewarding voting.


### Validator Admission Ticket (VAT)

While Alpenglow can in principle scale to large numbers of validators, to
simplify the implementation we want to limit the number of validators to at most
2,000. For example, we would like certificates to fit inside a single UDP
message. Because of this, Alpenglow enforces a strict limit and only admits the
2,000 highest staked validators. 2,000 is well above the numbers we see
currently, so we will likely not reach this limit in the near future.

Currently, validators must post their vote on the blockchain for every slot, and
they pay about 1 SOL per day in vote transaction fees. With Alpenglow, votes
will not go on chain. However, to maintain the present economic equilibrium,
initially we will reproduce the current set of incentives. For this reason, we
introduce a new validator admission ticket (VAT). Before being admitted to an
epoch, the VAT is deducted from each validator’s account. If the account is
insufficiently funded, the validator is removed from the active set. The VAT
will be 80% of the cost of today’s vote fees, in other words, initially about
0.8 SOL per day (or 1.6 SOL per epoch). In contrast to today, the whole VAT will
be burned to limit inflation. We leave proposals to make this mechanism adaptive
and more fitting to future SIMDs.


## Alternatives Considered

The following alternatives were considered:

- DAG-based Consensus
  - Pros: In the common case, bandwidth use is lower by the factor of the data
    expansion ratio.
  - Cons: worse latency, difficult to fully use bandwidth, huge paradigm shift
- No Single-Round Finality
  - Pros: simpler, 33% byzantine fault tolerance
  - Cons: worse latency, bigger concentrated stake problem, only 33% crash
    resilience


## Impact

The most visible change will be that optimistic confirmation is superseded by
faster (actual) finality.

Validator operators should also see their resource usage (including bandwidth
usage and compute) drop after the migration to Alpenglow.


## Security Considerations

The white paper provides proofs for safety and liveness under byzantine faults.

When instantiated with SHA256 for hashing and BLS12-381 for aggregate
signatures, the desired security level of 128-bits is achieved.

Alpenglow features a distinctive 20+20 security model. While it does not have
the 33% byzantine security that can be achieved with two-round voting protocols,
we believe that the one-round voting and the 40% crash failure resilience is
worth the tradeoff. 


## Drawbacks

The main drawback is the risk related to implementing a big protocol change.
Migrating to Alpenglow will be challenging.


## Backwards Compatibility

Incompatible. Alpenglow completely replaces the old consensus protocol with all
its voting logic.


## Bibliography

1. *Kniep, Sliwinski, Wattenhofer*, **Solana Alpenglow Consensus: Increased
   Bandwidth, Reduced Latency v1.1**, 2025,
   [https://www.anza.xyz/alpenglow-1-1](https://www.anza.xyz/alpenglow-1-1)