---
simd: '0089'
title: Programify Feature Gate Program 
authors:
  - Joe Caulfield
category: Standard
type: Core
status: Accepted
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
coordination. This makes upgrading the program cumbersome.

With the Feature Gate program instead implemented as a Core BPF program, any
changes need only be done once, eliminating this duplication of work.

## New Terminology

- Feature Gate program: The Core BPF program that all feature accounts will be
  assigned to, with address `Feature111111111111111111111111111111111111`.

## Detailed Design

A Core BPF program - the Feature Gate program - shall be placed at the address
of `Feature111111111111111111111111111111111111` using the process outlined in
[SIMD 0088](https://github.com/solana-foundation/solana-improvement-documents/pull/88).

The program will have no upgrade authority. If changes are required, for
essential network operations, the program will be updated using feature-gates.

The program shall initially be designed to support one instruction:
`RevokePendingActivation`. Any other instructions or functionality this program
may support in the future will be proposed and discussed separately.

When this instruction is invoked by a feature key-holder, the program will
reallocate the account to zero, assign it back to the System Program, and defund
its lamports balance. As a result, the runtime will no longer recognize this
feature as pending, since it will no longer be owned by
`Feature111111111111111111111111111111111111`.

The creation of this program will not change the existing feature activation
process.

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
anyone and execute code - will be the owner of these accounts. This introduces
new risks that must be mitigated through careful implementation of Feature Gate
program functionality.

## Backwards Compatibility

This change is 100% backwards compatible with the existing feature activation
process. It *only* adds the ability to revoke pending activations.
