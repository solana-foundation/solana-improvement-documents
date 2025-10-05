---
simd: '0371'
title: Alpenglow migration
authors:
  - Carl Lin
category: Standard
type: Core
status: Review
created: 2025-10-05
feature: TBD
---

## Summary

Migrate to Alpenglow from TowerBFT

## Motivation

Migrating from TowerBFT to Alpenglow consensus requires a safe handoff mechanism that doesn't rollback TowerBFT confirmed user transactions.

## Dependencies

This proposal depends on the following accepted proposal:

- **[SIMD-0326]: Alpenglow**

    Requires Alpenglow to be implemented in order to migrate


## New Terminology

N/A

## Detailed Design

1. Pick a migration boundary slot N, preferably not at the beginning of an epoch. After slot N, we turn off all user transactions and enter vote-only mode.

2. After this migration boundary slot, for every fork, if there's ever two earliest consecutive blocks `M`,`M+1`, where block `M+1` includes `>90%` of votes for `M`, then record `M+1` as a potential "transition block". Note these votes must be included in the block, detecting them in gossip is not sufficient!

3. After a validator detects such a "transition block" `M+1`, they also cast a BLS finalization vote for `M+1`. Additionally, for all blocks `S` desecended from `M+1`, every time the validator casts a TowerBFT vote for `S` they also cast an additional BLS finalization vote for `S`.

4. If any block `F > T` receives a `>90%` certificate aggregated from votes in gossip or turbine, then `F` must be optimistically confirmed, and we call the certificate for this block `F` the "finalized migration certificate".

5. Anytime a correct validator receives such a "finalized migration certificate" that they either constructed themselves or received from another validator, they:
 a. Broadcast the certificate to all other validators via the Alpenglow all-to-all mechanism, which guarantees delivery system via its retry mechanism.
 b. Rollback/delete all blocks after the `transition block` `M+1`,,
 c. Start Alpenglow using `M+1` as the initial Alpenglow genesis block

6. Alternatively, anytime a correct validator that has not yet detected a "finalized migration certificate" receives a "finalized Alpenglow certificate" for block `X`:
 a. Repair/replay all the ancestors of `X`
 b. Start Alpenglow after cathcing up to `X`

6. Once the first Alpenglow finalization certificate is detected, validators can stop broadcasting this "finalized migration certificate" as the Alpenglow finalization certificate is sufficent proof of the cluster's successful migration.


Safety argument:

First note it's always safe to rollback to the "transition block" because we stopped packing user transactions after the migration slot boundary earlier.

Next we show that if a "finalized migration certificate" exists, then all correct validators should pick the same "transition block" to start Alpenglow.

1. If two "finalized migration certificate"'s exist for different blocks `F` and `F'` where `F' > F`, then `F'` must descend from `F`. This is because all blocks that receive `90%` votes are optimistically confirmed, and thus must be on the same fork under the guarantees of optimistic confirmation (with 90% threshold that's safe assuming <28% malicious).

This means any validators that vote for `F` or `F'` must observe the earlier blocks in the fork.

2. For every fork, the "transition block" `T` is unique because 
 a. The votes to qualify for the 90% threshold must be observable/packed in the transition block
 b. For each fork, there can only be one such "earliest" block that passes this
 qualification, which everyone can observe

From 1. and 2. together this means for any "finalized migration certificate" `F`, everybody who voted on `F` agrees on the transition block `T`

This implies that if a `90%` "finalized migration certificate" exists, and we assume at most 20% of the cluster is malicious, then at least `70%` correct nodes will have observed the transition block `T` and will migrate to Alpenglow upon constructing the certificate or receiving the certificate via broadcast.

This `70%` correct validators is then sufficient to run Alpenglow and produce a finalized Alpenglow block, which will induce a repair/transition from any other lagging validators.

## Alternatives Considered

Alpenswitch where we pick fixed slot intervals `N` at which to attempt to optimistically migrate to Alpenglow. On failure fallback to TowerBFT we try again at the next slot interval. This is more painful to implement because
of the transition back and forth between Alpenglow and TowerBFT

## Impact

Clients will run Alpenglow after the migration boundary

## Security Considerations

N/A

## Backwards Compatibility

This feature is not backwards compatible.