---
simd: '0088'
title: Enable Core BPF Programs
authors:
  - Joe Caulfield
category: Standard
type: Core
status: Draft
created: 2023-11-07
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal introduces the concept of Core BPF programs: programs which are
essential to network operations. Currently, these exist as built-in programs
known as "native" programs.

This SIM also details the process by which existing native programs shall be
ported to Core BPF, as well as the process for introducing brand-new Core BPF
programs.

## Motivation

BPF programs offer less complexity for other clients, such as Firedancer, since
developers will no longer have to keep up with program changes in their runtime
implementations. Instead, the program can just be modified once. 

For this reason, it makes sense to introduce the concept of Core BPF programs:
BPF programs the network depends on that should be treated with special care.

## Alternatives Considered

The alternative to Core BPF programs is to keep these essential programs as
native programs. This would mean each validator client implementation would have
to build and maintain these built-in programs with their runtime
implementations, including any future changes to these programs introduced via
SIMDs or other fixes.

## New Terminology

- `Core BPF Program`: A BPF program relied on by any part of the Solana stack,
  including (but not limited to) consensus, transaction processing, voting,
  staking, and account creation.

## Detailed Design

Core BPF programs in many ways will be designed no differently than any other
BPF program on Solana. However, some programs may require special privileges,
which is beyond the scope of this SIMD.

When an existing native program is being proposed to migrate to Core BPF, or
when a new Core BPF program is being introduced, at least one SIMD shall be
published outlining at least the following details:

- How any required special privileges will be granted to the program in its BPF
  form
- How this program's upgrades will be managed after it becomes Core BPF

**Migrating a native program to core BPF** shall consist of deploying a modifed
version of the native program to a new arbitrary address and using a feature
gate to move the modified program in place of the existing program.

The feature gate is required to circumvent normal transaction processing rules
and replace the contents of one account with another directly at the runtime
level.

In the context of this design, **source program** shall be the modified BPF
version of a native program intending to be moved in place of the existing
native program, while **target program** is that existing program.

The migration process must adhere to the following steps:

1. Verifiably build the modified program.
2. Generate a new keypair for the **source** program.
3. Deploy the program to the **source** address.
4. Generate a new keypair for the **feature gate**.
5. Create a new feature gate for replacing the **target** program with the
   **source** (modified) program.
6. Follow the existing process for activating features.

## Impact

With this change, validator clients would no longer be required to implement
changes to essential programs. Instead these programs could be modified just
once. This reduces some engineering overhead on validator teams.

## Security Considerations

This proposal establishes the concept of relying on BPF programs that are not
built into the runtime for essential cluster operations. Depending on how these
programs are elected to be upgraded, there are some obvious security
considerations around who can upgrade these programs and when. Any new Core BPF
program should follow a fully-fledged SIMD process addressing these concerns.

When it comes to migrating native programs to core BPF, this change introduces a
serious security consideration surrounding the replacement of an essential
program with the contents of another account. This is an extremely sensitive
process that must be handled with maximum caution. If a core program is modified
incorrectly, or somehow erased during migration, it could have immediate and
fatal consequences for the network.

## Backwards Compatibility

This proposal itself does not directly introduce any breaking changes. The code
introduced to migrate native programs to core BPF programs will exist off of the
runtime's "hot path" until it's actually used for a migration.

When a migration is conducted, the core BPF version will be more than backwards
compatible. It must provide the exact same results as the native program it aims
to replace.

However, once a native program has been migrated to core BPF, the process by
which this program is upgraded will not be backwards compatible. Core
contributors must follow the new process.
