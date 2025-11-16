---
simd: '0384'
title: Alpenglow migration
authors:
  - Kobi Sliwinski (Anza)
  - Ashwin Sekar (Anza)
  - Carl Lin (Anza)
category: Standard
type: Core
status: Review
created: 2025-10-21
feature: a1penGLz8Vm2QHYB3JPefBiU4BY3Z6JkW2k3Scw5GWP
---

## Summary

Migrate to Alpenglow from TowerBFT

## Motivation

Migrating from TowerBFT to Alpenglow consensus requires a safe handoff
mechanism that doesn't rollback TowerBFT confirmed user transactions.

## Dependencies

This proposal depends on the following accepted proposal:

- **[SIMD-0326]: Alpenglow**

    Requires Alpenglow to be implemented in order to migrate

- **[SIMD-0307]: Add Block Footer**

    Specifies `BlockMarker`, an means of disseminating metadata in a block


## New Terminology

Migration boundary slot:

- Slot at which Alpenglow migration begins
- After this boundary slot:
   - Turn off packing anything other than simple vote transactions in blocks.
   Any transactions not
     belonging to the vote program will cause a block to be marked dead during
     replay by all correct validators.
   - Core code will stop notifying RPC services of any optimistic confirmation/
   commitment updates.
   - TowerBFT will stop rooting blocks to prevent losing optimistically
     confirmed blocks that could qualify as the Alpenglow genesis blocks.

Alpenglow genesis block:

- The last TowerBFT block before the "migration boundary slot" which is the
parent of the first Alpenglow block. This is picked via the process described in
the "Detailed Design" section below

Strong optimistic confirmation:

- A block `B` is strong OC if there exists a block `T` such that `B` is the parent
  of `T` and `slot(B) + 1 = slot(T)` and `T` contains vote transactions for `B` from
  at least `82%` of stake.

## Detailed Design

### Migration Handoff

1. Pick a "migration boundary slot" (defined above) `S` as follows. Let `X` be
the rooted slot in
   which the feature flag is activated. Let the migration slot S be `X + 5000`,
   as to avoid the beginning of an epoch.

2. After the migration boundary slot `S`, wait for some block `B > S` to reach
   strong optimistic confirmation.

3. Find the latest ancestor block `G` of `B` from before the migration boundary
   slot `S`. This is the Alpenglow genesis block. Cast a BLS vote, the "genesis
   vote", for `G` via all to all. Note validators should have filled out
   their BLS keys prior to the feature flag activation, and will sign this
   genesis vote with this BLS key.

4. If we observe `>=82%` genesis votes for the ancestor block `G`, this
   consitutes the `genesis certificate`, and `G` is the genesis block for
   Alpenglow. Validators will periodically refresh genesis votes every
   `GENESIS_VOTE_REFRESH` = 400ms (i.e. once a slot) until this
   `genesis certificate` is observed. During this period they perform regular
   TowerBFT consensus for all blocks.

5. Anytime a correct validator receives a `genesis certificate` for a slot `G`
   (either constructed themselves, received through replaying a block, or received
   from all-to-all broadcast), they:
   - Verify the certificate against the BLS keys fr the epoch.
   - Broadcast the certificate to all other validators via the Alpenglow
     all-to-all mechanism. Validators will continually retry broadcasting this
     certificate every 10 seconds via the certificate pool standstill timer so
     long as a finalized Alpenglow certificate for a higher slot isn't detected.
   - We initialize the alpenglow `votor` module with `G` as genesis, and disable
     TowerBFT for any slots past `G`
   - In block production pack the `genesis certificate` as a `GenesisBlockMarker`
     for any blocks that are *direct* children of `G`. This means anybody
     replaying any of the initial Alpenglow blocks must see the
     `genesis certificate`.
   - Delete all blocks and shreds with slot greater than `slot(G)` from
   blockstore, and reset all associated state (replay, AccountsDB) to block
   `G`, as is currently done when we rollback duplicate blocks.
   - We re-enable packing non-vote transactions, enable Alpenglow rooting, and
   re-enable RPC commitment/confirmation reporting.

6. Anytime a validator receives a `genesis certificate` validated through
   *replaying* the header of a block, they store the certificate in a
   `migration success` off-curve account 
   `Pubkey::find_program_address(&["carlgration"], alpenglow::id())`. 
   This means all snapshots descended from the block will contain this account 
   and signal to validators that they should initiate Alpenglow after unpacking 
   the snapshot.

7. Alternatively, anytime a correct validator that has not yet detected a
   `genesis certificate`, but receives an Alpenglow finalization certificate for
   some block `X` that they can verify, they should repair/replay all the
   ancestors of `X`

8. Once an Alpenglow finalization certificate is received via all-to-all or via
   replaying a block, validators can stop broadcasting the genesis certificate
   as the Alpenglow finalization certificate is sufficent proof of the
   cluster's successful migration.

9. On validator restart from a snapshot, if the migration feature flag is
   active:
   - If the off-curve `migration success` account is empty in the snapshot, we
     enter step 1.
   - If the off-curve `migration success` account contains a certificate and the
     certificate is valid, immediately enter Alpenglow.

### GenesisBlockMarker

In order to disseminate the `genesis certificate` in the initial Alpenglow block
we add a new `BlockMarker` to the specification of SIMD-0307 with variant ID `3`:

```
GenesisBlockMarker:
+---------------------------------------+
| Genesis Slot                (8 bytes) |
+---------------------------------------+
| Genesis Block ID           (32 bytes) |
+---------------------------------------+
| BLS Signature             (192 bytes) |
+---------------------------------------+
| Validator bitmap length     (8 bytes) |
+---------------------------------------+
| Validator bitmap      (max 512 bytes) |
+---------------------------------------+

Total size: max 752 bytes
```

The full serialization of this component is:

```
+---------------------------------------+
| Entry Count = 0             (8 bytes) |
+---------------------------------------+
| Marker Version = 1          (2 bytes) |
+---------------------------------------+
| Variant ID = 3              (1 byte)  |
+---------------------------------------+
| Length = max 752            (2 bytes) |
+---------------------------------------+
| Genesis Slot                (8 bytes) |
+---------------------------------------+
| Genesis Block ID           (32 bytes) |
+---------------------------------------+
| BLS Signature             (192 bytes) |
+---------------------------------------+
| Bitmap length (max 512)     (8 bytes) |
+---------------------------------------+
| Validator bitmap      (max 512 bytes) |
+---------------------------------------+

Total size: max 765 bytes
```


#### Correctness argument:

First note it's always safe to rollback a block greater after the migration
slot boundary because we stopped packing user transactions.

Next we show that if two correct validators switch to Alpenglow, they must pick
the same genesis block `G`.

To switch to Alpenglow, both correct validators must observe optimistic
confirmation on some slots `B` and `B'` past the migration boundary. It's
guaranteed by optimistic confirmation that `B` and `B'` are on the same fork,
and must have the same ancestors. This means all correct validators must cast
a genesis vote on the same ancestor block.

Because there's at most `19%` malicious, it will be impossible to construct two
`>82%` genesis certificates, so all correct validators that switch must have
observed the same genesis certificate for the same block.

#### Liveness argument

1. First we show that eventually at least one correct validator should see an
   Alpenglow genesis certificate.

We assume the the cluster must eventually run under normal network conditions,
so blocks past the migration boundary slot `S` should be `strongly optimistically
confirmed`.

Next, until a migration certificate is observed, no correct validators will
migrate, so all correct validators will vote as normal and contribute to
optimistic confirmation. From the correctness argument, we know if a correct
validator casts a genesis vote, they must vote for the same Alpenglow genesis
block.

This means that eventually 82% of validators will cast a genesis vote for the
same genesis block. Because these genesis votes are reliably delivered via
all-to-all, some correct validator will eventually get a genesis certificate.

2. Next we show that once a correct validator migrates, then all correct
   validators will eventually migrate.

There are two ways for correct validators to migrate:

1. A genesis certificate
2. An Alpenglow finalization certificate

The first correct validator to migrate must have gotten a `82%` genesis
certificate. We assume at most `19%` of the cluster is Byzantine, then at least
this correct validator will continuosly broadcast the genesis certificate until
they see an Alpenglow finalization certificate. 

This means `82% - 19% = 63%` correct nodes will eventually receive
the genesis certificate and will migrate to Alpenglow upon receiving the
certificate via all-to-all broadcast which guaranteees delivery.

This `63%` correct validators is then sufficient to run Alpenglow and produce
a finalized Alpenglow block, which will induce a repair/transition from any
other correct/lagging validators.

Thus from 1. we know some correct validator must eventually migrate because
eventually they must receive a genesis certificate, then we know from 2 that
eventually all correct validators must migrate.

### Poh Migration

When switching to the first Alpenglow block, we want to deprecate Poh. This will
be done in a few steps to mitigate the amount of code changes:

1. Before the end of each Alpenglow block, set the bank tick height to
   `bank.max_tick_height() - 1`
2. Change tick producer to only make 1 ending tick per block, so that each bank
   will still think it has reached `bank.max_tick_height()`. This last tick is
   necessary to coordinate with banking stage and broadcast to properly end the
   packing/dispersion of a block. Eliminating it is possible, but a load of
   risky work.
3. Change `blockstore_processor::verify_ticks()` to turn off tick verification.

### Duplicate block handling

On all blocks descended from the Alpenglow genesis block:

1. Turn off tower duplicate block handling
2. Turn off epoch slots

## Alternatives Considered

Alpenswitch where we pick fixed slot intervals `N` at which to attempt to
optimistically migrate to Alpenglow. On failure fallback to TowerBFT we try
again at the next slot interval. This is more painful to implement because
of the transition back and forth between Alpenglow and TowerBFT

## Impact

Validators will run Alpenglow after the migration boundary

## Security Considerations

N/A

## Backwards Compatibility

This feature is not backwards compatible.
