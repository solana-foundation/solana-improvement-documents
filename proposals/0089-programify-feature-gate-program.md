---
simd: '0089'
title: Programify Feature Gate Program 
authors:
  - Joe Caulfield
category: Standard
type: Core
status: Draft
created: 2023-11-21
feature: (fill in with feature tracking issues once accepted)
supersedes: '0077'
---

## Summary

This proposal suggests replacing the non-existent native program at address
`Feature111111111111111111111111111111111111` with a Core BPF Program, as
described in
[SIMD 0088](https://github.com/solana-foundation/solana-improvement-documents/pull/88).

Feature accounts are already assigned the owner program address
`Feature111111111111111111111111111111111111`. Deploying a Core BPF program at
this address would provide engineers with the capability to revoke pending
feature activations.

**Note:** The process by which core contributors *activate* features would
remain completely unchanged.

## Motivation

Currently, a feature is queued for activation by a keypair holder creating an
empty account and assigning it to the
`Feature111111111111111111111111111111111111` program.

Because there is no actual program implementation at this address, the queuing
is irreversible; if the runtime knows about a feature gate at some address, it
will activate it at the next epoch boundary. This means there is no recourse in
the case of a mistaken queuing, discovery of a bug, or simply a desire to manage
the cadence and schedule of activations.

A fully-implemented Core BPF program would take ownership of those accounts and
support revoking queued features, giving engineers more flexibility and
safeguards.

## Alternatives Considered

The Feature Gate program could instead be implemented as a built-in native
program, rather than a Core BPF program. However, this would mean any changes to
the program would need to be implemented by all validator clients in
coordination. This makes upgrading the program cumbersome and potentially
dangerous.

With the Feature Gate program instead implemented as a Core BPF program, the
program could be upgraded through the official feature-gate process described
in
[SIMD 0088](https://github.com/solana-foundation/solana-improvement-documents/pull/88).
Like all Core BPF programs, changes to the program only need to be done once
and are protected by the feature activation process.

Another alternative to a Core BPF program is a standard, upgradeable BPF program
whose upgrade authority is controlled by a multisig, with key-holders from each
client team - Solana Labs, Jito, Jump, and possibly Syndica. This arrangement
would make gating upgrades behind feature gates much more difficult. 

## New Terminology

- Feature Gate program: The Core BPF program that all feature accounts will be
  assigned to, with address `Feature111111111111111111111111111111111111`.
- "Revoke" or "revoke pending activation": The act of reallocating a feature
  account's data to zero, assigning it to the System Program, and defunding its
  lamports balance - effectively removing it from the runtime's recognized set
  of pending feature activations.

## Detailed Design

The program would initially be designed to support one instruction:
`RevokePendingActivation`. Any other instructions or functionality this program
may support in the future is outside the scope of this SIMD.

As mentioned above under "New Terminology", when this instruction is invoked by
a feature key-holder, the program will reallocate the account to zero, assign it
back to the System Program, and defund its lamports balance. As a result, the
runtime will no longer recognize this feature as pending, since it will no
longer be owned by `Feature111111111111111111111111111111111111`.

Consider the instruction as it may appear in the Feature Gate program:

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

The non-existent program at `Feature111111111111111111111111111111111111` can be
considered analogous to a no-op native program. Thus, the official processes
outlined in
[SIMD 0088](https://github.com/solana-foundation/solana-improvement-documents/pull/88)
for migrating a native program to Core BPF and upgrading a Core BPF
program can be used to enable this new program.

Consider the following steps to activate Feature Gate:

1. Migrate the native no-op program at
   `Feature111111111111111111111111111111111111` to a Core BPF no-op.
2. Upgrade the Core BPF no-op to add the `RevokePendingActivation` instruction.

Executing these two steps would effectively activate Feature Gate without any
changes to existing processes.

## Impact

Core contributors are positively impacted by this change, since the ability to
revoke pending feature activations is a significant security advantage.

There is otherwise no change to the activation process whatsoever. This includes
queuing features for activation with the CLI and the timing of their activation
by the runtime.

## Security Considerations

Currently the accounts used for feature-gating are owned by a program ID that
does not have any implementation. This means that there is no on-chain authority
that can modify feature accounts once they've been created under
`Feature111111111111111111111111111111111111`. This allows the runtime to
confidently update their state upon activation.

With this proposal, a live BPF program - which can accept instructions from
anyone and execute code - will be the owner of these accounts. This creates some
risk if _both_ the program's processor code as well as a secure system for
upgrading the program are not properly managed.

However, since this program would be Core BPF, its upgrades are protected by the
feature gate process. Thoroughly reviewed and safe processor code should
mitigate any new risks associated with this change.

## Backwards Compatibility

This change is 100% backwards compatible with the existing feature activation
process. It *only* adds the ability to revoke pending activations.
