---
simd: '0371'
title: Alpenglow migration
authors:
  - Kobi Sliwinski (Anza)
  - Ashwin Sekar (Anza)
  - Carl Lin (Anza)
category: Standard
type: Core
status: Review
created: 2025-10-21
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

1. Pick a migration boundary slot `S`, preferably not at the beginning of an epoch. After slot `S`:
a. Turn off packing user transactions in blocks and enter vote-only mode.
b. Turn off optimistic confirmation/commitment reporting over RPC.
c. Turn off rooting blocks in TowerBFT to prevent losing optimistically confirmed blocks that could qualify as the Alpenglow genesis blocks.

2. After this migration boundary slot, wait for some block `B` to be optimistically confirmed with >= 82% of the stake voting.

3. Find the latest ancestor block `G` of `B` from before the migration boundary slot `S`. Cast a BLS vote (the genesis vote) for `G` via all to all

4. If we observe `> 82%` genesis votes for the ancestor block `G`, this consitutes the `genesis certificate`, and `G` is the genesis block for Alpenglow.

5. Anytime a correct validator receives a `genesis certificate` for a slot `B`(either constructed themselves, received through replaying a block, or received from all-to-all broadcast), they:
 a. Broadcast the certificate to all other validators via the Alpenglow all-to-all mechanism, which guarantees delivery system via its retry mechanism.
 b. We initialize the alpenglow `votor` module with `G` as genesis, and disable TowerBFT for any slots past `G`
 c. In block production pack the `genesis certificate` in the headers of any blocks that are children of `G`. This means anybody replaying any of the initial Alpenglow blocks must see the `genesis certificate`.
 d. Delete all blocks with slot greater than `slot(G)`.
 e. We exit vote only mode, enable Alpenglow rooting, and re-enable RPC commitment/confirmation reporting.

6. Anytime a validator receives a `genesis certificate` validated through *replaying* the header of a block, they store the certificate in an off-curve account if that account is empty. This means all snapshots descended from the block will contain this account and signal to validators that they should initiate Alpenglow after unpacking the snapshot.

7. Alternatively, anytime a correct validator that has not yet detected a `genesis certificate`, but receives an Alpenglow finalization certificate for some block `X`, they should repair/replay all the ancestors of `X`

8. Once an Alpenglow finalization certificate is received via all-to-all or via replaying a block, validators can stop broadcasting the genesis certificate as the Alpenglow finalization certificate is sufficent proof of the cluster's successful migration.


#### Correctness argument:

First note it's always safe to rollback a block greater after the migration slot boundary because we stopped packing user transactions.

Next we show that if two correct validators switch to Alpenglow, they must pick the same genesis block `G`.

To switch to Alpenglow, both correct validators must observe optimistic confirmation on some slots `B` and `B'` past the migration boundary. It's guaranteed by optimistic confirmation that `B` and `B'` are on the same fork, and must have the same ancestors. This means all correct validators must cast a genesis vote on the same ancestor block.

Because there's at most `19%` malicious, it will be impossible to construct two `>82%` genesis certificates, so all correct validators that switch must have observed the same genesis certificate for the same block.

#### Liveness argument

1. First we show that eventually at least one correct validator should see an Alpenglow genesis certificate.

We assume the the cluster must eventually run under normal network conditions, so blocks past the migration boundary slot `S` should be optimistically confirmed.

Next, Until a migration certificate is observed, no correct validators will migrate, so all correct validators will vote as normal and contribute to optimistic confirmation. From the correctness argument, we know if a correct validator casts a genesis vote, they must vote for the same Alpenglow genesis block.

This means that eventually 82% of validators will cast a genesis vote for the same genesis block. Because these genesis votes are reliably delivered via all-to-all, some correct validator will eventually get a genesis certificate.

2. Next we show that once a correct validator migrates, then all correct validators will eventually migrate.

There's two ways for correct validators to migrate:
1. A genesis certificate
2. An Alpenglow finalization certificate

The first correct validator to migrate must have gotten a `82%` genesis certificate. We assume at most `19%` of the cluster is Byzantine, then at least this correct validator will continuosly broadcast the genesis certificate until they see an Alpenglow finalization certificate. 

This means eventually `82% - 19% = 63%` correct nodes will eventually receive the genesis certificate and will migrate to Alpenglow upon receiving the certificate via all-to-all broadcast which guaranteees delivery.

This `63%` correct validators is then sufficient to run Alpenglow and produce a finalized Alpenglow block, which will induce a repair/transition from any other correct/lagging validators.

Thus from 1. we know some correct validator must eventually migrate because eventually they must receive a genesis certificate, then we know from 2 that eventually all correct validators must migrate.

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