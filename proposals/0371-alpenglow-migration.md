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

### Migration Handoff

1. Pick a migration boundary slot `S`, preferably not at the beginning of an epoch. After slot `S`, we turn off all user transactions and enter vote-only mode, where the only votes being made are aggregatable BLS votes

2. After this migration boundary slot, wait for some block `B` to be optimistically confirmed with >= 82% of the stake voting. These votes are
aggregated into a migration certificate.

3. Anytime a correct validator receives such a migration certificate for a slot `B`( either constructed themselves or received from another validator), they:
 a. Broadcast the certificate to all other validators via the Alpenglow all-to-all mechanism, which guarantees delivery system via its retry mechanism.
 b. Find the latest ancestor `B' < S` of `B`. Delete all blocks with slot greater than `slot(B')`.
 c. Start Alpenglow using `B'` as the initial Alpenglow genesis block, and packing the migration certificate in any direct children of `B'`.

6. Alternatively, anytime a correct validator that has not yet detected a a migration certificate receives a "finalized Alpenglow certificate" for some block `X`:
 a. Repair/replay all the ancestors of `X`
 b. Start Alpenglow after catching up to `X`

6. Once the first Alpenglow finalization certificate is detected, validators can stop broadcasting the migration certificate as the Alpenglow finalization certificate is sufficent proof of the cluster's successful migration.


Correctness argument:

First note it's always safe to rollback a block greater after the migration slot boundary because we stopped packing user transactions.

Next we show that if two correct validators get migration certificates they should pick the same block to start Alpenglow.

Say two correct validators get migration certificates for some blocks `B` and `B'`. It's guaranteed by optimistic confirmation that `B` and `B'` are on the same fork, and must have the same ancestors. This means both will pick the same ancestor before the migraiton slot boundary as the Alpenglow genesis.

Liveness argument

We show that if a correct validator migrates, then all correct validators will migrate.

If a `82%` migration certificate exists, and we assume at most `19%` of the cluster is Byzantine, then at least `63%` correct nodes will have observed the transition block `T` and will migrate to Alpenglow upon receiving the certificate via all-to-all broadcast which guaranteees delivery.

This `63%` correct validators is then sufficient to run Alpenglow and produce a finalized Alpenglow block, which will induce a repair/transition from any other correct/lagging validators.

For paper, see here.

### Poh Migration

When switching to the first Alpenglow block, we want to deprecate Poh. This will
be done in a few steps to mitigate the amount of code changes:

1. Before the end of each Alpenglow block, set the bank tick height to `bank.max_tick_height() - 1`
2. Change tick producer to only make 1 ending tick per block, so that each bank will still think it has reached `bank.max_tick_height()`. This last tick is necessary to coordinate with banking stage and broadcast to properly end the packing/dispersion of a block. Eliminating it is possible, but a load of risky work.
3. Change `blockstore_processor::verify_ticks()` to turn off tick verification.

### Duplicate block handling

1. Turn off tower duplicate block handling
2. Turn off epoch slots

## Alternatives Considered

Alpenswitch where we pick fixed slot intervals `N` at which to attempt to optimistically migrate to Alpenglow. On failure fallback to TowerBFT we try again at the next slot interval. This is more painful to implement because
of the transition back and forth between Alpenglow and TowerBFT

## Impact

Validators will run Alpenglow after the migration boundary

## Security Considerations

N/A

## Backwards Compatibility

This feature is not backwards compatible.