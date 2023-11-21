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
essential to network operations. Currently, these are embedded programs known as
"native" programs.

This SIMD also outlines the processes by which engineers will manage upgrades to
these enshrined Core BPF programs, as well as migrate existing native programs
to core BPF.

## Motivation

With the advent of Program Runtime v2 and Firedancer, native programs will
eventually become BPF programs.

Program Runtime v2 will introduce a brand-new ABI for on-chain programs, and the
native programs will need to be revised to support it.

BPF programs offer less complexity for other clients, such as Firedancer, since
developers will no longer have to keep up with program changes in their runtime
implementations. Instead, the program can just be modified once. 

For these reasons, it makes sense to introduce the concept of Core BPF programs:
BPF programs the network depends on that should be upgraded with a special
process.

With an explicit process in place for upgrading Core BPF programs, engineers can
safely manage changes to essential programs through feature gates. The feature
gate process requries incremental activations on each cluster: testnet, then
devnet, then mainnet-beta. This is extremely valuable for ensuring a change has
been safely integrated and will not have negative effects on mainnet-beta.

This same explicit process can be used to migrate existing native programs to
their new Core BPF versions.

## Alternatives Considered

An alternative approach to managing upgrades to essential core programs could be
to make these core BPF programs _upgradeable_ and establish a multi-signature
upgrade authority. This authority could be comprised of key-holders from each
validator client: Solana Labs, Jito, Jump, and possibly Syndica (Sig).

However, this approach adds additional layers of complexity around upgrading
these programs behind a feature-gate.

## New Terminology

- `Core BPF Program`: A **non-upgradeable** BPF program relied on by any part of
  the Solana stack, including (but not limited to) consensus, transaction
  processing, voting, staking, and account creation.
- `Target Program`: The core program intending to be changed by some
  feature activation.
- `Source Program`: The modified version of a core program intending to be moved
  in place of the existing core program.

## Detailed Design

Core BPF programs shall be non-upgradeable BPF programs deployed with
`BPFLoader2111111111111111111111111111111111`.

Core BPF programs must only be modified through feature gates.

**Upgrading a core BPF program** shall consist of deploying the modified program
source code to a new arbitrary address as a **non-upgradeable** BPF program and
creating a feature gate to, using the runtime, move the modified program in
place of the existing program. 

Note, since this deployed program will be deployed using the non-upgradeable BPF
loader (`BPFLoader2111111111111111111111111111111111`), it will consist of only
a program account, with no additional program _data_ account.

This process must adhere to the following steps:

1. Verifiably build the modified program.
2. Generate a new keypair for the **source** program.
3. Deploy the program to the **source** address.
4. Generate a new keypair for the **feature gate**.
5. Create a new feature gate for replacing the **target** program with the
   **source** (modified) program.
6. Follow the existing process for activating features.

An additional optional field could be added to feature gate issues for the
**target program** being upgraded.

**Migrating a native program** to a core BPF program shall consist of following
the exact steps outlined above with the modified BPF version of the program.
This will effectively replace the program at the target address with its core
BPF implementation.

Some additional checks should be run when doing a migration instead of an
upgrade, such as:

- Validating the native program's owner is
  `NativeLoader1111111111111111111111111111111`
- Ensuring the core BPF program has the correct state

## Impact

This proposed change would result in a new process for upgrading core programs.
Although this process will still require feature gates as it does now, the
change is handled completely differently.

The act of deploying the modified program and using a runtime feature gate to
move it into the proper account will be an entirely new way of upgrading
programs that were previously native.

Core contributors will have to become comfortable with this process.

## Security Considerations

This proposal provides a secure means for upgrading core BPF programs - a
process that will be increasingly valuable as new core BPF programs are created.

However, this change also introduces a critical security consideration
surrounding the replacement of a core program with the contents of another
account. This is an extremely sensitive process that must be handled with
maximum caution.

If a core program is modified incorrectly, or somehow erased during migration,
it could have immediate and fatal consequences for the network.

## Backwards Compatibility

This feature itself does not directly introduce any breaking changes.

When this proposed process is used to migrate a native program to core BPF, that
program will be completely backwards compatible.

However, once a program has been migrated to core BPF, the process by which this
program is upgraded will not be backward compatible. Core contributors must
follow the new process.
