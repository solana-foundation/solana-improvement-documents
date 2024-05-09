---
simd: '0149'
title: Migrate Snapshot Serialized Epoch Stakes
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Draft
created: 2024-05-09
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Migrate bank snapshots to a new "epoch stakes" field in order to store
additional stake state needed for calculating partitioned rewards.

## Motivation

In order to properly support recalculating partitioned rewards (SIMD-0118)
when rebooting from a snapshot in the middle of the rewards distribution
period at the beginning of an epoch, additional stake state should be
stored in the epoch stakes snapshot field.

Since the currently used epoch stakes field doesn't support versioning, we
propose migrating the snapshot format to use a new epoch stakes field which is
versioned and supports storing the required stake state.

## New Terminology

NA

## Alternatives Considered

We have discussed the following alternative approach:

1. The missing stake state information can be retrieved from the current epoch's
stakes cache instead of epoch stakes. However this alternative is risky because
the current epoch's stake cache is be updated during rewards distribution with
newly rewarded stake accounts and possibly newly created stake accounts as well.

## Detailed Design

### Serialized Data Changes

To be more specific, the missing stake information is the `credits_observed`
field from the `Stake` struct:

```rust
struct Stake {
    delegation: Delegation,
    credits_observed: u64,
}
```

The currently used `epoch_stakes` field in the bank snapshot serializes a map of
epochs to `EpochStakes` structs which have special logic for serializing stake
state:

```rust
struct (BankFieldsToDeserialize | BankFieldsToSerialize) {
    ..
    epoch_stakes: HashMap<Epoch, EpochStakes>,
    ..
}

struct EpochStakes {
    #[serde(with = "crate::stakes::serde_stakes_enum_compat")]
    stakes: Arc<StakesEnum>,
    total_stake: u64,
    node_id_to_vote_accounts: Arc<NodeIdToVoteAccounts>,
    epoch_authorized_voters: Arc<EpochAuthorizedVoters>,
}

enum StakesEnum {
    Accounts(Stakes<StakeAccount>),
    Delegations(Stakes<Delegation>),
}
```

When serializing `EpochStakes` in snapshots, all `StakesEnum` variants first map
stake entry values to their `Delegation` value before serialization. The goal of
this proposal is to migrate to a new epoch stakes field which maps stake entry
values to their full `Stake` value before serialization so that
`credits_observed` will be be included in the snapshot and available after
snapshot deserialization.

The proposed `new_epoch_stakes` bank snapshot field will instead serialize a map
of epochs to `VersionedEpochStakes` structs which can be updated in the future
to serialize different information if needed. This field will be appended to the
end of the serialized bank snapshot:

```rust
struct (BankFieldsToDeserialize | BankFieldsToSerialize) {
    ..
    new_epoch_stakes: HashMap<Epoch, VersionedEpochStakes>,
}

enum VersionedEpochStakes {
    Current {
        stakes: Stakes<Stake>,
        total_stake: u64,
        node_id_to_vote_accounts: Arc<NodeIdToVoteAccounts>,
        epoch_authorized_voters: Arc<EpochAuthorizedVoters>,
    },
}
```

### Snapshot Update Rollout

Handling snapshot format changes is always a delicate operation to coordinate
given that old software releases will not be able to deserialize snapshots from
new software releases properly. The rollout will require two phases:

1. Introduce support for deserializing the migrated epoch stakes field 
2. Enable serializing epoch stakes to the new field and phase out the old field

During the first phase, validator software will be updated to attempt to
deserialize the new epoch stakes field appended at the end of the bank snapshot.
If the field doesn't exist, validators will continue using the old deserialized
field. If the new epoch stakes field is deserialized successfully, the epoch
stakes entries from this field will be merged with entries from the old field.

During the second phase, validator software will be updated to start serializing
epoch stakes entries to the new epoch stakes field. Note, however, that there
are 3 different epoch stakes entry variants:

1. Entries created during epoch boundaries which have full stake account data
2. Entries deserialized from the old snapshot epoch stakes field which only have
    stake delegation state.
3. Entries deserialized from the new snapshot epoch stakes field which have full
    stake state.

Only variants 1 and 3 can be serialized into the new epoch stakes field so any
variant 2 epoch stakes entries will continue being serialized into the old epoch
stakes field.

We propose adding a new feature gate `migrate_epoch_stakes_snapshot_field` to
facilitate the migration by activating phase 2 so that the implementation for
both phases can be added to the next beta release.

## Impact

No major impact beyond backwards compatibility concerns. Snapshots will
be a few MB larger than before.

## Security Considerations

Missing or corrupted epoch stakes entries caused by faulty snapshot migration
can cause validators to fork off from the cluster or cause the cluster to lose
consensus if sufficient stake is affected.

## Backwards Compatibility

Snapshot changes must be made in a backwards compatible way. Handling
compatibility is thoroughly discussed in the proposal above.

## Open Questions

NA
