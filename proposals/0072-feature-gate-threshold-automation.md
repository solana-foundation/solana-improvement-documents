---
simd: '0072'
title: Feature Gate Threshold Automation
authors:
  - Tyera Eulburg
  - Joe Caulfield
category: Standard
type: Core
status: Draft
created: 2023-10-11
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This SIMD outlines a proposal for automating the feature activation process
based on a stake-weighted support threshold, rather than manual human action.

## Motivation

Feature gates wrap new cluster functionality, and typically change the rules of
consensus. As such, a feature gate needs to be supported by a strong majority
of cluster stake when it is activated, or else it risks partitioning the
network. The current feature-gate system comprises two steps:

1. An individual key-holder queues a feature gate for activation
2. The runtime automatically activates the feature on the next epoch boundary

The key-holder is the one who (with the help of the solana-cli) assesses the
amount of stake that recognizes the feature and decides whether it is safe to
activate. This is obviously brittle and subject to human error.

This SIMD proposes that the runtime itself replaces the human to assess the
amount of stake that supports a particular queued feature and only
automatically activates it when a preset threshold is met. 

## New Terminology

- Feature Gate program: The BPF program that will own all feature accounts and
possess the capability to revoke them.
- Feature-Request account: The PDA under the Feature Gate program used to track
features requested for activation.
- Feature-Queue account: The PDA under the Feature Gate program used to track
features queued for activation that must have support signaled by nodes.

## Detailed Design

Before this SIMD is published, we will deploy a BPF program to manage feature
activations, which will allow us to revoke pending feature activations
(previously not possible).
The feature activation process will otherwise be unchanged, so no SIMD is
required.
This program will be upgradeable; the authority and process through which it
will be upgraded is outside the scope of this SIMD. Since this SIMD requires an
update to the program, the upgrade details will be determined in parallel.

The initial Feature Gate Program can be found in this pull request to the
Solana Program Library:
<https://github.com/solana-labs/solana-program-library/pull/4987>.
The runtime changes and feature activation that will enable this program can be
found in this feature tracking issue:
<https://github.com/solana-labs/solana/issues/33547>.

### Overview

Consider two arbitrary versions of the Solana runtime, where the newer version
has more features in its feature set.

| v1.14.25  | v1.16.0   |
| --------- | --------- |
| Feature A | Feature A |
|           | Feature B |
| Feature C | Feature C |
|           | Feature D |

The runtime should activate each of these features when a strong majority of
the network supports it. “Support” for a feature means that a node is running a
version of the Solana runtime where the code required for the feature is
present. For instance, on a cluster with 50% of stake running v1.14.25 and 50%
of stake running v1.16.0, only Features A and C should be eligible for
activation.

This SIMD proposes 3 steps over the span of one epoch to determine which
feature gates are queued for activation and assess stake support:

1. On an epoch boundary, the runtime generates a list of feature-gates queued
for activation.
2. Validators signal which of the queued feature-gates they support.
3. On the next epoch boundary, the runtime activates the feature-gates that
have the necessary stake support

### Representing the Current Feature Set

As mentioned above, a BPF program is being deployed that will replace the
non-existent account `Feature111111111111111111111111111111111111`, which is
the owner of all feature accounts.

The Feature Gate program will utilize two PDAs to track the authoritative
feature set queued for activation. One PDA (the Feature-Request PDA) will
store newly requested feature activations and the other (the Feature-Queue
PDA) will store the previous epoch’s requested activations.

When a new epoch begins, the Feature-Request PDA will be an empty list. When
key-holders submit the CLI command to activate a feature, the Feature Gate
Program appends the feature ID to the Feature-Request set at the same time as
creating the feature account . Revoked features are removed from the
Feature-Request list. At the end of the epoch, the Feature-Request set is
written to the feature queue PDA, where it becomes immutable. The
Feature-Request PDA is then reset to an empty list again.

The newly created immutable Feature-Queue can then be used by nodes to signal
support for potential activation in the next epoch.

Note that for any given epoch, the epoch will begin with an immutable set of
previously requested feature activations, and end with the activation of those
with enough support. This process takes one epoch. Therefore, it will take at
least one more epoch than it currently does from the time a key-holder submits
a feature for activation via CLI to when it is actually activated.

### Signaling Support for Features

With an on-chain reference point to determine the features queued for
activation, nodes can signal support for specific features in the queue.

A node signals its support for a feature set by sending a transaction to the
Feature Gate program, signed by their authorized voter, to store a record of
their supported feature set.
This supported feature set would be a bit mask of the list of queued features,
and it would be stored in a PDA derived from their vote address.
Nodes simply examine the Feature-Queue list, and mark a 1 for any features they
have in their feature set. The rest are marked 0.

In the previous example, a node running v1.14.25 would signal the following bit
mask:

```
1   0   1   0 
```

Similarly, a node running v1.16.0 would signal:

```
1   1   1   1 
```

Validators send these transactions signaling their support for a queued set of
features on the first slot of every epoch and on startup after any reboot.

Note: If a feature is revoked before it is moved to the immutable Feature-Queue
list, it is simply removed from the Feature-Request list and never presented to
nodes. However, if a feature is revoked after it becomes part of the immutable
Feature-Queue list, it remains in the list, and nodes still signal their
support if the feature ID is present in their software. However, the runtime
won’t activate this feature if its corresponding feature account no longer
exists on-chain.
This is how the runtime currently handles features and can be observed here:
<https://github.com/solana-labs/solana/blob/170478924705c9c62dbeb475c5425b68ba61b375/runtime/src/bank.rs#L8119-L8150>.

### Activating Supported Features

Once the epoch concludes, the runtime uses the validator support signals to
activate the proper features.

To do this, the runtime walks all of the PDAs derived from the vote accounts,
examines their bit-masks, and sums the stake supporting each feature-gate to
determine which feature gates meet the desired threshold of supported stake.

For the initial implementation, the threshold will be hard-coded to 95% of
stake, but future iterations on the Feature Program could allow feature
key-holders to set a custom threshold for each feature gate
(including 0%, ie. –yolo).

Once a list of stake-supported features is generated, the runtime rechecks the
Feature accounts in case any have been revoked. Then the runtime processes the
activations using the existing feature logic.

At this point, the runtime clears the existing validator support PDA accounts
and returns to step 1 to generate the next list of queued features.

### Conclusion

The current familiar process of activating a feature with
`solana feature activate` will now merely suggest this feature be considered
for activation by the aforementioned mechanism. In other words, it’s no longer
a guarantee that your feature will be activated when you run this command.

Instead, the runtime will decide through stake whether or not a feature should
be activated. This will be especially useful in a world where two separate
clients will aim to push features and want to agree on their activation.

## Alternatives Considered

### Representing the Current Feature Set (Alternative)

An alternate approach to the Feature Gate program’s on-chain storage is to
instead use one single PDA to store the queue and reset it every epoch. This
would allow us to use one less account and one less epoch, but it would make
things trickier when it comes to the timing of validators signaling support for
features.

For example, a key-holder might submit a feature for activation at a point late
in the epoch, so validators would need to be able to signal support for this
feature whenever it is submitted. This means validators would have to
consistently check the feature queue for changes, and possibly issue more
signals in the epoch.

This approach may increase the total required work for a node in a given epoch.

### Signaling Support for Features (Alternative)

Rather than sending a transaction to the Feature Gate program, a node could
instead signal the bit mask in their block header whenever they are the leader.

This would introduce a change to the block header and remove this information
from being publicly and transparently on-chain.

## Impact

This new process for activating features directly impacts core contributors and
validators.

Core contributors will no longer bear the responsibility of ensuring the proper
stake supports their feature activation. However, it will change the process
by which they can override this stake requirement to push a feature through.

Validators will be responsible for signaling their vote using a transaction
which they've previously not included in their process. They also will have a
more significant impact on feature activations if they neglect to upgrade their
software version.

## Security Considerations

This proposal increases security for feature activations by removing the human
element from ensuring the proper stake supports a feature.

However, it also adds a dependency on a BPF program and the data within its
owned accounts to perform feature activations, which previously has not been
the case. This may introduce some security concerns around this relationship
between the runtime and the Feature Gate program.

This proposal could also potentially extend the length of time required for
integrating feature-gated changes, which may include security fixes. However,
the feature-gate process is relatively slow in its current state, and neither
the current process or this proposed process would have any implications for
critical, time-sensitive issues.
