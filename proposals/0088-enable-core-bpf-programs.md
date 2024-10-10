---
simd: '0088'
title: Enable Core BPF Programs
authors:
  - Joe Caulfield
category: Standard
type: Core
status: Accepted
created: 2023-11-07
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal introduces the concept of Core BPF programs: programs which are
essential to network operations. Currently, these exist as built-in programs
known as "native" programs.

This SIMD details the process by which existing native programs can be
ported to Core BPF, as well as the process for introducing brand-new Core BPF
programs.

## Motivation

BPF programs offer less complexity than native programs for other clients, such
as Firedancer, since developers will no longer have to keep up with program
changes in their runtime implementations. Instead, the program can just be
updated once.

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

When an existing native program is being proposed to be migrated to Core BPF,
or when a new Core BPF program is being introduced, at least one SIMD shall be
published outlining at least the following details:

- The interface for the program
- A precise and complete specification of its behavior
- How any required special privileges will be granted to the program in its BPF
  form
- Whether or not this program will be an upgradeable or non-upgradeable BPF
  program
- How this changes to this program will be managed after it becomes BPF

**Migrating a native program to core BPF** shall consist of creating a buffer
account, owned by the BPF Upgradeable Loader, containing the program's desired
upgrade authority and the ELF bytes of its BPF implementation. A feature gate
is used to create the BPF program accounts, replacing the existing native
program at its original address with a legitimate BPF Upgradeable program.

In the slot immediately following the feature activation, the program will *not*
be invocable. This status will last one slot, then the program will be fully
operational.

No program IDs for existing native programs are changed by this migration
process.

In the context of this design, **target program** refers to an existing native
program, while **source buffer account** refers to the buffer account
containing the BPF implementation of the target native program.

The migration process must adhere to the following steps:

1. Verifiably build the ELF of the BPF implementation.
2. Generate a new keypair for the source buffer account.
3. Create the buffer account and write the ELF bytes to it.
4. Generate a new keypair for the feature gate.
5. Create a new feature gate to replace the target program with the source
   buffer.
6. Follow the existing process for activating features.

## Impact

With this change, validator clients would no longer be required to implement
changes to essential programs. Instead these programs could be updated just
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
process that must be handled with maximum caution. If a core program is
reimplemented incorrectly, or somehow erased during migration, it could have
immediate and fatal consequences for the network.

## Backwards Compatibility

This proposal itself does not directly introduce any breaking changes. The code
introduced to migrate native programs to core BPF programs will exist off of the
runtime's "hot path" until it's actually used for a migration.

When a migration is conducted, the BPF version of a native program will be
absolutely backwards compatible *functionally*. Its BPF version must provide the
exact same results as the original native program it aims to replace.

However, since BPF programs cannot precisely match the compute meter and other
resource limits of their original native counterparts, some of these metrics may
be slightly different when a native program becomes BPF, thereby affecting
backwards compatibility in that regard.

Additionally, once a native program has been migrated to core BPF, the process
by which this program is upgraded will not be backwards compatible. Core
contributors must follow the upgrade process outlined in each program's
migration SIMD.
