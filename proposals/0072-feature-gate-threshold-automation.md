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

- **Feature Gate program:** The Core BPF program introduced in
  [SIMD 0089](./0089-programify-feature-gate-program.md)
  that will own all feature accounts.
- **Staged Features PDA:** A PDA under the Feature Gate program used to track
  features submitted for activation per epoch.
- **Validator Support Signal PDA:** A PDA under the Feature Gate program used to
  track a validator's support signal bitmask, which signals which features they
  support.
- **Get Epoch Stake Syscall:** The new syscall introduced in
  [SIMD 0133](./0133-syscall-get-epoch-stake.md)
  that returns the current epoch stake for a given vote account address.

## Detailed Design

This proposal outlines a new feature activation process. The new process
includes changes to the runtime's feature activation process as well as the
Core BPF Feature Gate program proposed in
[SIMD 0089](./0089-programify-feature-gate-program.md).

The new process will utilize the Feature Gate program to enable the runtime to
activate staged features that meet the necessary stake support while preventing
the activation of those that do not.

Two new instructions and one new type of PDA will be added to the Feature Gate
program. They are detailed in this proposal.

The new process is comprised of the following steps:

1. **Feature Creation:** Contributors create feature accounts as they do now.
2. **Staging Features for Activation:** In some epoch `N-1`, a multi-signature
   authority stages for activation some or all of the features created in step
   1, to be activated at the end of the *next epoch* (epoch `N`).
3. **Signaling Support for Staged Features:** During the next epoch (epoch `N`),
   validators signal which of the staged feature-gates they support in their
   software.
4. **Feature Activation:** At the end of epoch `N`, the runtime activates the
   feature-gates that have the required stake support.

### Step 1: Feature Creation

The first step is creation of a feature account, done by submitting a
transaction containing System instructions to fund, allocate, and assign the
feature account to `Feature111111111111111111111111111111111111`.

This step is unchanged from its original procedure.

### Step 2: Staging Features for Activation

A multi-signature authority, comprised of key-holders from Anza and
possibly other validator client teams in the future, will have the authority to
stage created features for activation.
In the future, this authority could be replaced by validator governance.

The multi-signature authority stages a feature for activation by invoking a new
Feature Gate program instruction: `StageFeatureForActivation`. This instruction
expects the Staged Features PDA (defined below) to either exist or be allocated
with sufficient space and owned by the Feature Gate program, in order to
initialize state. 

The `StageFeatureForActivation` instruction is structured as follows:

- Data: None
- Accounts:
  - Feature account
  - Staged Features PDA: writable
  - Multi-signature authority: signer

Note that features can only be staged in the epoch prior to the target
activation epoch. This means a feature staged by invoking
`StageFeatureForActivation` in epoch `N-1` is scheduled for activation (through
the process below) at the end of epoch `N`. This is checked by the `Clock`
sysvar.

When a feature is staged for activation, initial stake support is zero.

#### Staged Features PDA State

A Staged Features PDA will be created for each epoch in which features are
staged to be activated. If no features are staged for a given epoch, that
epoch's corresponding Staged Features PDA will never be initialized and thus
will not exist.

These PDAs will not be garbage collected and can be referenced for historical
purposes.

The address of the Staged Features PDA for a given epoch is derived as follows,
where `epoch` is a `u64` serialized to eight little-endian bytes:

```
"staged_features" + <epoch>
```

The data for the Staged Features PDA will be structured as follows:

```c
#define FEATURE_ID_SIZE 32
#define MAX_FEATURES 8

/**
 * A Feature ID and its corresponding stake support, as signalled by validators.
 */
typedef struct {
    /** Feature identifier (32 bytes for public key). */
    uint8_t feature_id[FEATURE_ID_SIZE];
    /** Stake support (u64 serialized to little-endian). */
    uint8_t stake_support[8];
} FeatureStake;

/**
 * Staged features for activation. 
 */
typedef struct {
    /**
     * Features staged for activation at the end of the current epoch, with
     * their corresponding signalled stake support.
     */
    FeatureStake features[MAX_FEATURES];
} StagedFeatures;
```

As depicted in the above layout, a maximum of 8 features can be staged for a
given epoch.

### Step 3: Signaling Support for Staged Features

With an on-chain reference point to determine the features staged for activation
for a particular epoch, nodes will signal their support for the staged features
supported by their software.

A node signals its support for staged features by invoking another new Feature
Gate program instruction: `SignalSupportForStagedFeatures`. This instruction
expects the Validator Support Signal PDA (defined below) to either exist or be
allocated with sufficient space and owned by the Feature Gate program, in order
to initialize state.

The `SignalSupportForStagedFeatures` instruction is structured as follows:

- Data: A `u8` bit mask of the staged features.
- Accounts:
  - Staged Features PDA: writable
  - Validator Support Signal PDA: writable
  - Vote account
  - Authorized voter: signer

The authorized voter signer must match the authorized voter stored in the vote
account's state.

The `SignalSupportForStagedFeatures` instruction processor will provide the
vote account's address to the `GetEpochStake` syscall to retrieve the stake
delegated to that vote account for the epoch. Then, using the `1` values
provided in the bitmask (defined below), the processor will add this stake value
to each corresponding feature ID's stake support in the Staged Features PDA.

A node's stake support for features is always accounted for using their most
recently sent bitmask. Each time a node sends a bitmask, if a previous bitmask
was already sent for that node, their previous bitmask is first used to deduct
stake support before adding stake support for the features signalled by the new
bitmask.

Similar to the `StageFeatureForActivation` instruction, the Clock sysvar will be
used to ensure the Staged Features PDA corresponding to the *current* epoch `N` was
provided.

If a node does not send this instruction successfully during the current epoch,
their stake is not tallied. This is analogous to a node signalling support for
zero features.

#### Signal Bitmask

A bit mask is used as a compressed ordered list of indices. This has two main
benefits:

- Minimizes transaction size for nodes, requiring only one byte to describe
  256 bytes (8 * 32) of data. The alternative would be sending `n` number of
  32-byte addresses in each instruction.
- Reduce compute required to search the list of staged features for a matching
  address. The bitmask's format provides the Feature Gate program with the
  indices it needs to store supported stake without searching through the staged
  features for a match.

A `1` bit represents support for a feature. For example, for staged features
`[A, B, C, D, E, F, G, H]`, if a node wishes to signal support for all features
except `E` and `H`, their `u8` value would be 246, or `11110110`.

#### Validator Support Signal PDA State

A Validator Support Signal PDA will be created for each vote account. It will
store the node's submitted bitmasks.

As mentioned previously, a node's most recently submitted bitmask is considered
their signal for the epoch. Validator Support Signal state allows one bitmask
per epoch, but can store multiple epochs of historical bitmasks. This is useful
for querying stake support post-epoch. When a new bitmask is submitted for an
epoch with an existing entry in the account state, it is overwritten.

These accounts are never garbage collected since they are reused every epoch.
This means nodes are only required to pay for rent-exemption once.

The address of the Validator Support Signal PDA (for a given epoch?) is derived
as follows, where `vote_address` is the 32 bytes of the validator's vote address.

```
"support_signal" <vote_address>
```

The data for the Validator Support Signal PDA will be structured as follows:

```c
#define MAX_SIGNALS 4

/**
 * A validator's support signal bitmask along with the epoch the signal
 * corresponds to.
 */
typedef struct {
    /**
     * The epoch the support signal corresponds to (u64 serialized to
     * little-endian).
    */
    uint8_t epoch[8];
    /** The support signal bitmask. */
    u8 signal;
    /** Padding for 8-byte alignment. */
    uint8_t _padding[7];
} SupportSignalWithEpoch;

/**
 * A validator's support signal bitmasks with their corresponding epochs.
 */
typedef struct {
    /**
     * The support signal bitmasks with their corresponding epochs.
     */
    SupportSignalWithEpoch signals[MAX_SIGNALS];
} ValidatorSupportSignal;
```

As depicted in the above layout, a validator's signal can be stored (and
queried) for up to 4 epochs.

### Step 4: Feature Activation

At the end of the epoch, the runtime loads the Staged Features PDA for the
current epoch and calculates the stake - as a percentage of the total epoch
stake - in support of each feature to determine which staged features to
activate.

Every feature whose stake support meets the required threshold must be
activated. This threshold will be hard-coded in the runtime to 95% initially,
but future iterations on the process could make this threshold configurable.

Features can be revoked at any point up until this step (staged or unstaged). If
a feature is revoked, nodes may still have signalled support for it, but the
runtime will not activate the feature since the account will not exist on-chain.
For more information, see the
[Feature Gate Program's](https://github.com/solana-program/feature-gate)
`RevokePendingActivation` instruction, as proposed in
[SIMD 0089](./0089-programify-feature-gate-program.md).

If a feature is not activated, it must be resubmitted according to Step 2. If it
is revoked, it must be resubmitted according to Step 1.

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

## Security Considerations

This proposal increases security for feature activations by removing the human
element from ensuring the proper stake supports a feature.

This proposal could also potentially extend the length of time required for
integrating feature-gated changes, which may include security fixes. However,
the feature-gate process is relatively slow in its current state, and neither
the current process or this proposed process would have any implications for
critical, time-sensitive issues.

