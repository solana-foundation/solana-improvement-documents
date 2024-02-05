---
simd: '0072'
title: Feature Gate Threshold Automation
authors:
  - Tyera Eulburg
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
  [SIMD 0089](https://github.com/solana-foundation/solana-improvement-documents/pull/89)
  that will own all feature accounts.
- **Staged Features PDA:** The PDA under the Feature Gate program used to track
  features submitted for activation for a particular epoch.
- **Support Signal PDA:** The PDA under the Feature Gate program used to store
  a bit mask of the staged features a node supports.
- **Feature Tombstone:** A static address with no on-chain account for assigning
  accounts under, effectively "archiving" them and removing them from the
  Feature Gate program's owned accounts.

## Detailed Design

The new process is comprised of the following steps:

1. **Feature Creation:** Contributors create feature accounts as they do now.
2. **Staging Features for Activation:** In some epoch N, a multi-signature
   authority stages for activation some or all of the features created in step
   1.
3. **Signaling Support for Staged Features:** During the next epoch (epoch N+1),
   validators signal which of the staged feature-gates they support in their
   software.
4. **Activation and Garbage Collection:** At the end of epoch N+1, the runtime
   activates the feature-gates that have the necessary stake support. At the
   same time, the runtime also archives activated feature accounts and PDAs no
   longer required by the process.

### Step 1: Feature Creation

The first step is creation of a feature account, done by submitting a
transaction containing System instructions to fund, allocate, and assign the
feature account to `Feature111111111111111111111111111111111111`.

### Step 2: Staging Features for Activation

A multi-signature authority, comprised of key-holders from Solana Labs and
possibly other validator client teams in the future, will have the authority to
stage created features for activation.
In the future, this authority could be replaced by validator governance.

The multi-signature authority stages a feature for activation by invoking the
Feature Gate program's `StageFeatureForActivation` instruction. This instruction
expects:

- Data: The feature ID
- Accounts:
  - Staged Features PDA: writable
  - Multi-signature authority: signer 

`StageFeatureForActivation` will add the provided feature ID to the **next
epoch's** Staged Features PDA.

The Staged Features PDA for a given epoch stores a list of all feature IDs that
were staged for activation during the previous epoch. This list shall have a
maximum length of 8 (ie. `[Pubkey; 8]`).

The proposed seeds for deriving the Staged Features PDA are provided below,
where the `<epoch>` is the epoch during which the contained feature IDs will be
assessed for stake support and considered for activation.

```
"staged_features" + <epoch>
```

### Step 3: Signaling Support for Staged Features

With an on-chain reference point to determine the features staged for activation
for a particular epoch, nodes can signal their support for the staged features
supported by their software.

A node signals its support for staged features by invoking the Feature Gate
program's `SignalSupportForStagedFeatures` instruction. This instruction
expects:

- Data: A `u8` bit mask of the staged features.
- Accounts:
  - Support Signal PDA: writable
  - Vote account: signer

A `1` bit represents support for a feature. For example, for staged features
`[A, B, C, D, E, F, G, H]`, if a node wishes to signal support for all features
except `E` and `H`, their `u8` value would be 246, or `11110110`.

A node's submitted bit mask is then stored in a Support Signal PDA derived from
that node's vote address. The proposed seeds are defined below.

```
"support_signal" + <vote_address>
```

Nodes should send a transaction containing this instruction at some arbitrary
point during the epoch at least 128 slots before the end of the epoch and on
startup after any reboot.

Note: If a feature is revoked, the list of staged features will not change, and
nodes may still signal support for this feature. However, the runtime will not
activate this feature if its corresponding feature account no longer exists
on-chain.

### Step 4: Activation and Garbage Collection

During the epoch rollover, the runtime uses the validator support signals to
determine which staged features to activate.

To do this, the runtime walks all of the vote accounts, derives their Support
Signal PDA to read their bit mask, and tallies up the total stake support for
each staged feature. The runtime will also zero-out each bit mask, resetting
each Support Signal PDA for the next epoch.

Only features whose stake support meets the required threshold are activated.
This threshold shall be set to 95% initially, but future iterations on the
process could allow feature key-holders to set a custom threshold per-feature.

If a feature is not activated, either because it has been revoked or it did not
meet the required stake support, it must be resubmitted according to Step 2.

To ensure this new process doesn't overload the Feature Gate program's owned
accounts, during the activation stage, garbage collection will:

- Archive any activated feature accounts
- Archive this epoch's Staged Features PDA

The runtime "archives" an account by assigning it to the Feature Tombstone.

Created features that were not staged for activation or did not meet the
required stake support will not be garbage collected.

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

