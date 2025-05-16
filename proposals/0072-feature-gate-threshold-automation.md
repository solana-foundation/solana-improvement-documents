---
simd: '0072'
title: Feature Gate Threshold Automation
authors:
  - Tyera Eulberg
  - Joe Caulfield
category: Standard
type: Core
status: Idea
created: 2024-01-25
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This SIMD outlines a proposal for automating the feature activation process
based on a stake-weighted support threshold, rather than manual human action.

With this new process, contributors no longer have to assess stake support for a
feature before activation. Instead, the assessment is done by the runtime.

## Motivation

Feature gates wrap new cluster functionality, and typically change the rules of
consensus. As such, a feature gate needs to be supported by a strong majority
of cluster stake when it is activated, or else it risks partitioning the
network. The current feature-gate process involves two steps:

1. An individual key-holder stages a feature gate for activation
2. The runtime automatically activates the feature on the next epoch boundary

The key-holder is the one who *manually* (with the help of the solana-cli)
assesses the amount of stake that recognizes the feature and decides whether
it is safe to activate. This is obviously brittle and subject to human error.

If instead the runtime was to assess this stake support for activating a
feature, this would eliminate the key-holder's responsibility to asses stake
support, reducing the risk of human error.

In a world where multiple clients will aim to push features and seek to agree on
their activation, a more automated and secure process will be extremely
beneficial.

## New Terminology

- **Staging authority**: The authority who must sign to stage a given feature
  for activation, or unstage that feature.
- **Signal account (PDA)**: A PDA under the Feature Gate program used to
  prevent double-signaling.

## Detailed Design

A new feature activation process, comprised of the following steps:

1. Contributor creates a feature account and configures the feature's staging
   authority.
2. In some epoch `N-1`, the staging authority stages the feature for activation.
3. During the next epoch `N`, validators signal which of the staged features
   they support.
4. At the end of epoch `N`, the runtime activates the features with enough stake
   support.

### New Feature Account State

A new version of feature account state will track feature status, staging
authority, and stake support. Since the current feature state is a Rust
`Option<u64>`, we will use a `2` for the first byte to denote feature state v2.
The new state - `FeatureV2` will use a constant account data length of 49
bytes, corresponding to the length of the longest variant.

```
[0, ...]: v1 None (total len 9)
[1, ...]: v1 Some (total len 9)
[2, ...]: v2 (total len 49)
```

```rust
enum FeatureV2 {
    /// Feature is inactive and will not be included in any support signals.
    Inactive {
        /// The authority who can mark this feature as staged for activation.
        staging_authority: Pubkey,
    },
    /// The feature is staged for activation, and will be included in the
    /// support signals for this feature's `effective_epoch`.
    Staged {
        /// The epoch this feature became staged for signaling support.
        ///
        /// If a feature is staged in epoch N-1, this is epoch N. Since a
        /// feature may go several epochs before finally being activated, this
        /// field represents the first epoch the feature became eligible for
        /// support signaling.
        effective_epoch: u64,
        /// The authority who can mark this feature as inactive (same as the
        /// original `staging_authority`).
        staging_authority: Pubkey,
        /// How much stake support has been signaled for this feature.
        stake_support: u64,
    },
    /// The feature is active.
    Active {
        /// The epoch in which this feature was activated.
        activation_epoch: u64,
    }
}
```

A runtime feature-gate will direct the runtime to ignore any v1 feature state
and only recognize feature state v2. Migration of feature state v1 to v2 will be
supported by the Feature Gate program, but only for inactive features.

During step 1, contributors will still fund, allocate, and assign new feature
accounts to `Feature111111111111111111111111111111111111` as they do now, but an
additional step to initialize feature state v2 will be required. This is done
via the new Feature Gate program instruction `InitializeFeature`.

Note that the feature account must be allocated and rent-exempt for 49 bytes,
otherwise the initializing instruction will fail.

```
Instruction: InitializeFeature
- Data:
    - 1 byte: Instruction discriminator (`0`)
    - 32 bytes: Address of the staging authority
- Accounts:
    - [s, w]: Feature account
```

Another instruction will be available to update the staging authority:
`UpdateStagingAuthority`.

```
Instruction: UpdateStagingAuthority
- Data:
    - 1 byte: Instruction discriminator (`1`)
    - 32 bytes: Address of the new staging authority
- Accounts:
    - [s]: Current staging authority
    - [w]: Feature account
```

Step 2 begins whenever the staging authority invokes the Feature Gate program
instruction `StageFeature`, which simply sets the feature's status to
`Staged` with initial stake support of zero.

```
Instruction: StageFeature
- Data:
    - 1 byte: Instruction discriminator (`2`)
- Accounts:
    - [s]: Staging authority
    - [w]: Feature account
```

This operation can be reversed anytime, as long as the feature still has
`Staged` status, via the `UnstageFeature` instruction.

```
Instruction: UnstageFeature
- Data:
    - 1 byte: Instruction discriminator (`3`)
- Accounts:
    - [s]: Staging authority
    - [w]: Feature account
```

### Signaling Support

Each epoch, all nodes must load all feature accounts with feature state v2 and
signal support for any staged features they wish to see activated. Staged
features they do not wish to activate can be intentionally omitted.

Support is signaled through one or more transactions, depending on the number of
staged features in a given epoch, which contain multiple Feature Gate program
`SignalSupportForStagedFeature` instructions.

```
Instruction: SignalSupportForStagedFeature
- Data:
    - 1 byte: Instruction discriminator (`4`)
- Accounts:
    - [s]: Authorized voter
    - [ ]: Validator vote account
    - [w]: Signal account
    - [w]: Feature account
```

The authorized voter signer must match the authorized voter stored in the vote
account's state.

The program processor will use the `SolGetEpochStake` syscall to retrive the
corresponding epoch stake for the provided vote account, and add it to the
feature's stake support.

The signal account is a PDA derived from the epoch, feature ID and vote address
to prevent double-signaling.

```
epoch_le_bytes + feature_id + vote_address
```

It contains just 8 bytes for the little-endian `u64` stake amount. These signal
accounts can serve as historical records for which nodes signaled support for
which features. They also serve as a source of truth for withdrawing support via
the `WithdrawSupportForStagedFeature` instruction, which will deduct the stored
stake value in the signal account from the feature's stake support and clear the
signal account.

```
Instruction: WithdrawSupportForStagedFeature
- Data:
    - 1 byte: Instruction discriminator (`5`)
- Accounts:
    - [s]: Authorized voter
    - [ ]: Validator vote account
    - [w]: Signal account
    - [w]: Feature account
```

Although there is no requirement for features to be staged in the previous
epoch, staging might occur late in an epoch, after most support signals have
already been cast. This would result in not enough stake support until the next
epoch arrives and new signals are cast.

### Activation

At the end of the epoch, the runtime loads all staged feature accounts and
calculates their stake support as a percentage of the cluster-wide stake. Only
features whose account state has status `Staged` at the epoch rollover will be
evaluated. 

Every feature whose stake support meets the required threshold will be
activated. This threshold will be hard-coded in the runtime to 95% initially,
but future iterations on the process could make this threshold configurable.

Features without the required stake support will remain `Staged` until they
acquire enough stake support or are manually unstaged.

## Alternatives Considered

## Impact

This new process for activating features directly impacts core contributors and
validators.

Core contributors will no longer bear the responsibility of ensuring the proper
stake supports their feature activation. However, this proposal does not include
a mechanism for overriding or customizing the stake requirement. This capability
should be proposed separately.

Validators will be responsible for signaling their vote using a transaction
which they've previously not included in their process. They also will have a
more significant impact on feature activations if they neglect to upgrade their
software version.

Note that revoking pending features, as enabled by
[SIMD 0089](./0089-programify-feature-gate-program.md) is unchanged by this
proposal, and also applies to `Staged` features.

## Security Considerations

This proposal increases security for feature activations by removing the human
element from ensuring the proper stake supports a feature.

This proposal could also potentially extend the length of time required for
integrating feature-gated changes, which may include security fixes. However,
the feature-gate process is relatively slow in its current state, and neither
the current process or this proposed process would have any implications for
critical, time-sensitive issues.

