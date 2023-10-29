---
simd: '0077'
title: Programify Feature Gate Program
authors:
  - Joe Caulfield
  - Tyera Eulberg
category: Standard
type: Core
status: Draft
created: 2023-10-26
feature: #33547
---

## Summary

### Roadmap

This is SIMD 1/3 expected for **Multi-Client Feature Gates**. See
<https://github.com/solana-foundation/solana-improvement-documents/issues/76>

**Goals:**

- Decentralized control over queuing new runtime features
- Automatic feature activation selection based on stake weight with supporting
  software
- Decentralized control of the mechanism itself

**Resulting Architecture:**

1. 👉 **Feature Creation:** Features can be created by anyone. Each is owned by
  an upgradeable BPF program at `Feature111111111111111111111111111111111111`
2. **Feature Queuing:** A governance process nominates features that should be
  queued for activation
3. **Feature Recognition & Activation:** Features are activated based on stake
  support of nodes who recognize the feature in their software version

### Proposal

This SIMD outlines a proposal to replace the non-existent program at address
`Feature111111111111111111111111111111111111`, which is the owner of all
feature accounts, with an upgradeable BPF program.

It defines the program's initial functionality - which consists solely of the
capability to revoke pending feature activations - and an initial upgrade
control process for managing upgrades of the program.

Important note: the process by which core contributors *activate* features
would remain completely unchanged by this SIMD.

## Motivation

The complete Multi-Client Feature Gate architecture - mentioned under "Roadmap"
above - will require several changes to the way feature accounts are handled.
The most obvious approach is to place a program in charge of these accounts.
An upgradeable BPF program presents a more logical solution over a native
program for this task, since it provides a built-in system for upgrades.

In the case of this particular proposal, this BPF program will provide
engineers the capability to revoke pending feature activations.

Currently, a feature is queued for activation by a keypair holder creating an
empty account and assigning it to the
`Feature111111111111111111111111111111111111` program. Because there is no
actual program at that address, the queuing is irreversible; if the runtime
knows about a feature gate at that address, it will activate it at the next
epoch boundary. This means there is no recourse in the case of a mistaken
queuing, discovery of a bug in the feature's code, or simply a desire to manage
the cadence and schedule of activations.

A fully-fledged BPF program would take ownership of those accounts and support
revoking queued features, giving engineers more flexibility and safeguards.

## Alternatives Considered

The Feature Gate program could instead be a native program, rather than a BPF
program. However, this would mean any changes to the program would need to be
implemented by all validator clients in coordination. This makes upgrading the
program to support the complete Multi-Client Feature Gate architecture
cumbersome and potentially dangerous.

However, one of the main benefits gained by instead opting for a native program
would be an easier ability to upgrade via feature gate.

Another alternative considered is to use the deployment process outlined in the
"Deploying the Program" section under "Detailed Design" to upgrade the program
behind feature gates in the future. In short, contributors would stage changes
to the program in another account and create a runtime feature gate to swap
this upgraded version of the program into the current program's place.

Note this may render the multi-signature program upgrade authority setup
useless, except for less common use cases, such as a critical upgrade that
could not be done through a feature gate.

## New Terminology

- Feature Gate program: The BPF program that will own all feature accounts.
- “Revoke” or “revoke pending activation”: The act of reallocating a feature
  account’s data to zero, assigning it to the system program, and defunding
  its lamports balance - effectively removing it from the runtime’s recognized
  set of features pending activation.

## Detailed Design

The design for this proposal consists of three components:

- Deployment of the program by the runtime
- Revoking pending features using the BPF program
- Upgrade control process for the BPF program

### Deploying the Program

In order to deploy an upgradeable BPF program to the address at
`Feature111111111111111111111111111111111111`, a runtime change is required.
This change would allow the runtime, upon feature activation, to move an
already deployed upgradeable BPF program into the account at address
`Feature111111111111111111111111111111111111`.

For maximum security, the initial program can be a no-op program with an
intentionally large allocation (to allow for larger programs in the future).

Once the program is moved into place, the program can be upgraded using the
conventional `BpfUpgradeableLoader` method to include the revoke functionality
defined below.

The specific no-op program to be initially moved into
`Feature111111111111111111111111111111111111` by the runtime should be
verifiably built and then deployed to devent, testnet, and mainnet-beta.

### Revoking Pending Features

As mentioned above under “New Terminology”, the act of revoking a pending
feature activation consists of reallocating a feature account’s data to zero,
assigning it to the system program, and defunding its lamports balance. This
causes the feature account to be “revoked” since the runtime will no longer
detect it as an account owned by `Feature111111111111111111111111111111111111`.

When a core contributor executes the `solana feature activate` command, a
signature from the feature keypair is required to activate it, since its state
will change. Similarly, we can require the same feature keypair’s signature to
revoke said feature.

Consider the instruction as it would appear in the Feature Gate Program:

```rust
pub enum FeatureGateInstruction {
    /// Revoke a pending feature activation.
    ///
    /// A "pending" feature activation is a feature account that has been
    /// allocated and assigned, but hasn't yet been updated by the runtime
    /// with an `activation_slot`.
    ///
    /// Features that _have_ been activated by the runtime cannot be revoked.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w+s]`    Feature account
    ///   1. `[w]`      Destination (for rent lamports)
    RevokePendingActivation,
}
```

When this instruction is invoked with the proper signature, the feature account
would be reallocated, defunded, and returned to the System Program, like so:

```rust
/* Checks */

let new_destination_lamports = feature_info
    .lamports()
    .checked_add(destination_info.lamports())
    .ok_or::<ProgramError>(FeatureGateError::Overflow.into())?;

**feature_info.try_borrow_mut_lamports()? = 0;
**destination_info.try_borrow_mut_lamports()? = new_destination_lamports;

feature_info.realloc(0, true)?;
feature_info.assign(&system_program::id());
```

### Controlling the Program Upgrades

Because the Feature Gate program requires special runtime support, upgrading
the Feature Gate program will initially be controlled by a 2-of-3
multi-signature authority, consisting of key-holders distributed across
validator-client teams.

- Solana Labs: 1 key-holder
- Jump: 1 key-holder
- Jito: 1 key-holder

Only when 2 out of 3 key-holders have authorized an upgrade will the Feature
Gate program be upgraded.

**Note:** This includes the upgrade required to upgrade the initial no-op
program to the first released Feature Gate Program supporting revoke.

## Impact

Core contributors are positively impacted by this change, since the ability to
revoke pending feature activations is a significant security advantage.

There is otherwise no change to the activation process whatsoever. This
includes queuing features for activation with the CLI and the timing of their
activation by the runtime.

This proposal also increases decentralized control over some components of the
feature activation process, but can be more decentralized in the future.

## Security Considerations

Currently the accounts used for feature-gating are owned by a program ID that
does not exists. This means that there’s no on-chain authority that can modify
feature accounts once they’ve been created. This allows the runtime to
confidently update their state upon activation.

With this proposal, a live BPF program - which can accept instructions from
anyone and execute code - will be the owner of these accounts. This does create
some risk if *both* the program’s processor code *and* its upgrade authority
are not properly managed.  But thoroughly reviewed and safe processor code, as
well as a decentralized system for upgrading the program, will together
mitigate these new risks as much as possible.

## Backwards Compatibility

As mentioned under "Summary", the process by which core contributors *activate*
features would remain completely unchanged by this SIMD.

This SIMD only *adds* the capability to revoke pending activations, so it's
completely backwards compatible.