---
simd: '0072'
title: Feature Gate Threshold Automation
authors:
  - Tyera Eulberg
  - Joe Caulfield
category: Standard
type: Core
status: Draft
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
- **Staged Features PDA:** The PDA under the Feature Gate program used to track
  features submitted for activation.
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

Two new instructions, as well as two types of PDAs, will be added to the
Feature Gate program. They are detailed in this proposal.

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
expects:

- Data: The feature ID
- Accounts:
  - Staged Features PDA: writable
  - Multi-signature authority: signer
  - Payer: optional signer

A PDA will be created for each epoch in which features are staged to be
activated. If no features are staged for a given epoch, that epoch's
corresponding PDA is not created.

These PDAs will not be garbage collected and can be referenced for historical
purposes.

When the first feature for an epoch is staged, the PDA is created. The
`StageFeatureForActivation` processor will debit from the payer account enough
lamports to allocate the new Staged Features PDA. A payer account must be
provided for the first staged feature.

The address of the Staged Features PDA for a given epoch is derived as follows,
where `epoch number` is a little-endian `u64`:

```
"staged_features" + < epoch number >
```

The data for the Staged Features PDA will be structured as follows:

```c
#define FEATURE_ID_SIZE 32
#define MAX_FEATURES 8

/**
 * A Feature ID and its corresponding stake support, as signalled by validators.
 */
typedef struct {
    /** Feature identifier (32 bytes). */
    uint8_t feature_id[FEATURE_ID_SIZE];
    /** Stake support (little-endian u64). */
    uint8_t stake[8];
} FeatureStake;

/**
 * Staged features for activation. 
 */
typedef struct {
    /**
     * Features staged for activation at the end of the current epoch, with
     * their corresponding signalled stake support.
     */
    FeatureStake current_feature_stakes[MAX_FEATURES];
} StagedFeatures;
```

`StageFeatureForActivation` may only be invoked during epoch `N-1`, where `N` is
the epoch number used to derive the Staged Features program-derived address.
This is checked by the Feature Gate program using the Clock sysvar.

Features staged during epoch `N-1` are staged to be activated at the end of
epoch `N`.

`StageFeatureForActivation` will add the provided feature ID to the list with
stake support initialized to `0`.

As depicted in the above layout, a maximum of 8 features can be staged for a
given epoch.

### Step 3: Signaling Support for Staged Features

With an on-chain reference point to determine the features staged for activation
for a particular epoch, nodes will signal their support for the staged features
supported by their software.

A node signals its support for staged features by invoking another new Feature
Gate program instruction: `SignalSupportForStagedFeatures`. This instruction
expects:

- Data: A `u8` bit mask of the staged features.
- Accounts:
  - Staged Features PDA: writable
  - Vote account: signer

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

The `SignalSupportForStagedFeatures` instruction processor will provide the
vote account's address to the `GetEpochStake` syscall to retrieve the stake
delegated to that vote account for the epoch. Then, using the `1` values
provided in the bitmask, the processor will add this stake value to each
corresponding feature ID's stake support in the Staged Features PDA.

Similar to the `StageFeatureForActivation` instruction, the Clock sysvar will be
used to ensure the Staged Features PDA corresponding to the *current* epoch `N` was
provided.

If a node does not send this transaction successfully during the current epoch,
their stake is not tallied. This is analogous to a node signalling support for
zero features.

If a feature is revoked, the list of staged features will not change, and nodes
may still signal support for this feature. However, the runtime will not
activate this feature if its corresponding feature account no longer exists
on-chain.

### Step 4: Feature Activation

During the epoch rollover, the runtime must load the Staged Features PDA
and calculate the stake - as a percentage of the total epoch stake - in support
for each feature ID to determine which staged features to activate.

Every feature whose stake support meets the required threshold must be
activated. This threshold will be hard-coded in the runtime to 95% initially,
but future iterations on the process could allow feature key-holders to set a
custom threshold per-feature.

As mentioned previously, if a feature was revoked, it will no longer exist
on-chain, and therefore will be not activated by the runtime, regardless of
calculated stake support.

If a feature is not activated, either because it has been revoked or it did not
meet the required stake support, it must be resubmitted according to Step 2.

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

