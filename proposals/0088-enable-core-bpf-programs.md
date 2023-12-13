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

This proposal outlines the process by which engineers will manage upgrades to
enshrined Core BPF programs, as well as migrate existing native programs to core
BPF.

This SIMD also introduces the concept of Core BPF programs: programs which are
essential to network operations. Currently, these are embedded programs known as
"native" programs.

## Motivation

BPF programs offer less complexity for other clients, such as Firedancer, since
developers will no longer have to keep up with program changes in their runtime
implementations. Instead, the program can just be modified once. 

For this reason, it makes sense to introduce the concept of Core BPF programs:
BPF programs the network depends on that should be upgraded with a special
process.

## Alternatives Considered

An alternative approach to managing upgrades to Core BPF programs could be to
manually upgrade these programs behind a feature gate using the runtime. This
process is more complex and involves circumventing the established program
upgrade process.

It's possible that multi-sig program upgrades could be combined with
feature-gates, however this implementation would add additional layers of
complexity to the upgrade process.

## New Terminology

- `Core BPF Program`: An **upgradeable** BPF program relied on by any part of
  the Solana stack, including (but not limited to) consensus, transaction
  processing, voting, staking, and account creation.

## Detailed Design

Core BPF programs shall be upgradeable BPF programs deployed with
`BPFLoaderUpgradeab1e11111111111111111111111`.

The upgrade authority shall be a multi-sig comprised of keyholders from each
validator client. Right now, that list includes Solana Labs, Jito, and Jump.
This list can be updated in the future to include newer clients like Sig.

**Upgrading a core BPF program** shall consist of a coordinated effort amongst
core contributors from all validator clients. Similar to the feature gate
process, this upgrade should occur on testnet, then devnet, then mainnet-beta.

Upgraded versions of core BPF programs shall always be compiled using a
verifiable build process.

**Migrating a native program to core BPF** shall consist of deploying a modifed
version of the native program to a new arbitrary address as an **upgradeable**
BPF program and using a feature gate to move the modified program in place of
the existing program.

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

This also introduces a multi-sig process, which core contributors across all
validator teams must, in a timely manner, participate in.

## Security Considerations

This proposal establishes the concept of relying on BPF programs that are not
built into the runtime for essential cluster operations. With these programs
being upgradeable, there are some obvious security considerations around who can
upgrade these programs and when. With a proper multi-sig process in place, these
risks are mitigated.

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
