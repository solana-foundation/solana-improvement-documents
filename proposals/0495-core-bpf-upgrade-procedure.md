---
simd: '0495'
title: Core BPF Upgrade Procedure
authors:
  - Joe Caulfield (Anza)
category: Advisory
type: Advisory
status: Idea
created: 2026-03-18
feature: N/A
extends: SIMD-0088
---

## Summary

This SIMD describes the procedure for upgrading a Core BPF program that may
have been migrated from a native builtin or otherwise deployed on-chain as a
core program. Each individual Core BPF program upgrade may reference this
procedure when proposing an upgrade and specifying its corresponding feature
gate.

## Motivation

Core BPF programs are essential to network operations and may be deployed in
a number of ways, including migration from a native builtin (as described in
[SIMD-0088]) or direct deployment as an on-chain program. Regardless of how
they are deployed, these programs may have no upgrade authority or may
otherwise not be upgradeable through the standard Loader v3 `Upgrade`
instruction.

However, Core BPF programs still require updates. Bug fixes, new features
introduced via SIMDs, and protocol changes all necessitate a mechanism for
upgrading these programs through consensus.

This SIMD defines a feature-gated, runtime-level procedure for upgrading
Core BPF programs.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0088]: Enable Core BPF Programs**

    Defines the concept of Core BPF programs.

[SIMD-0088]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0088-enable-core-bpf-programs.md

## New Terminology

N/A

## Detailed Design

Core BPF programs are Loader v3 (BPF Upgradeable Loader) programs, as
specified in [SIMD-0088]. Upgrading a Core BPF program involves replacing
the contents of an existing program data account with new ELF bytes from a
source buffer account. The program account itself is not modified. A feature
gate triggers the upgrade at activation time.

### Preparation

Before an upgrade can occur, the following steps must be completed:

1. Verifiably build the ELF of the new BPF implementation using a verified
   build tool (e.g. `solana-verify`).
2. Compute the SHA-256 hash of the resulting ELF bytes.
3. Generate a new keypair for the source buffer account.
4. Create the buffer account and write the ELF bytes to it using the Loader
   v3 `Write` instruction. The buffer account must be owned by the BPF
   Upgradeable Loader (Loader v3).
5. Generate a new keypair for the feature gate.
6. Create a new feature gate to upgrade the target program with the source
   buffer, according to your validator implementation. The verified build
   hash from step 2 must be included in the upgrade configuration
   alongside the feature gate.
7. Follow the existing process for activating features.

### Source Buffer Validation

When the feature gate activates, the runtime must validate the source buffer
account before proceeding. The source buffer account:

- Must exist.
- Must be owned by the BPF Upgradeable Loader.
- Must deserialize as `UpgradeableLoaderState::Buffer`.

If any of these checks fail, the upgrade must not proceed.

### Verified Build Hash

Each Core BPF upgrade must include a verified build hash in its upgrade
configuration. The verified build hash is the SHA-256 hash of the expected
ELF bytes. This hash is computed during preparation and enshrined in the
validator client alongside the feature gate and source buffer address.

When the feature gate activates, after validating the source buffer account
structure, the runtime must:

1. Extract the ELF bytes from the source buffer account (all bytes after
   the `UpgradeableLoaderState::Buffer` metadata header, excluding
   trailing zero bytes).
2. Compute the SHA-256 hash of these bytes.
3. Compare the computed hash against the expected verified build hash.

If the hashes do not match, the upgrade must not proceed.

This ensures that the exact ELF bytes deployed in the buffer at preparation
time are the same bytes used during the upgrade. No party can alter the
buffer contents between preparation and feature activation without
invalidating the hash.

### Target Program Validation

The target Core BPF program must also be validated. The program account:

- Must exist.
- Must be owned by the BPF Upgradeable Loader.
- Must be executable.
- Must deserialize as `UpgradeableLoaderState::Program` with a
  `programdata_address` that matches the derived program data address for
  the program ID.

The program data account (derived from the program ID):

- Must exist.
- Must be owned by the BPF Upgradeable Loader.
- Must deserialize as `UpgradeableLoaderState::ProgramData`.

If any of these checks fail, the upgrade must not proceed.

### Upgrade Authority Matching

If the target program data account's `upgrade_authority_address` is `None`,
the source buffer's authority is ignored and the new program data account's
upgrade authority will also be set to `None`.

If the target program data account's `upgrade_authority_address` is `Some`,
the source buffer account's authority must match. If they do not match, the
upgrade must not proceed.

### Upgrade Result

The ELF bytes from the source buffer must be validated against the current
runtime environment before any accounts are modified. If validation fails,
the upgrade must not proceed.

Once validation passes, the following conditions must hold after the upgrade
completes:

- **Program account**: Unchanged. It retains its existing
  `UpgradeableLoaderState::Program` state, owner, and lamports.

- **Program data account**: Replaced at the same derived address. The new
  account contains `UpgradeableLoaderState::ProgramData` with the `slot`
  field set to the current slot and the same `upgrade_authority_address` as
  the previous program data account. The ELF bytes from the source buffer
  follow the metadata header. The account is owned by the BPF Upgradeable
  Loader and funded at the rent-exempt minimum for its new size.

- **Source buffer account**: Data and lamports cleared, assigned to System.

- **Capitalization**: The lamports from the old program data account and the
  source buffer account are burned. The new program data account is funded
  at the rent-exempt minimum for its new size. Total capitalization is
  adjusted by the difference between burned and funded lamports.

- **Accounts data size delta**: Updated to reflect the difference between
  the old total data size (old program data account + source buffer account)
  and the new total data size (new program data account).

### Program Availability

In the slot immediately following the feature activation, the program will
not be invocable. This status will last one slot, then the program will be
fully operational. The program ID does not change.

### Feature Gates

Each Core BPF program upgrade must be controlled by a feature gate, which
triggers the upgrade when activated.

## Alternatives Considered

### Standard Loader v3 Upgrade Instruction

Core BPF programs with `None` as their upgrade authority cannot use the
standard Loader v3 `Upgrade` instruction. Even if an upgrade authority were
set, using the standard instruction would require a single signer to hold
the authority, which is inappropriate for programs critical to network
operations. The feature-gated approach ensures upgrades go through the same
consensus process as other protocol changes.

### Governance-Based Upgrade Authority

An alternative would be to assign a multisig or governance program as the
upgrade authority. This was rejected because multisig operations are slow
and difficult to coordinate with the timing of feature gate activations,
which must occur at the exact same slot across all validators. It also
introduces additional complexity and dependencies on external programs for
what are consensus-critical operations.

## Impact

This proposal provides a clear, repeatable process for upgrading Core BPF
programs. Validator client teams can coordinate upgrades through the existing
feature gate process, ensuring all participants in the network agree on
program changes before they take effect.

## Security Considerations

Upgrading a Core BPF program replaces the executable code of a program that
the network depends on. This is an extremely sensitive operation that must be
handled with maximum caution. If a core program is upgraded incorrectly, it
could have immediate consequences for the network.

The feature-gated approach mitigates this risk by requiring the upgrade to go
through the standard feature activation process, which includes community
review, testnet validation, and coordinated mainnet activation.

The source buffer account must be deployed and verified on-chain before the
feature gate is activated. This allows the community to inspect and verify
the exact ELF bytes that will be deployed.

The verified build hash provides an additional layer of protection by
cryptographically binding the upgrade to a specific set of ELF bytes. Even
if the source buffer account's authority were compromised, the runtime will
reject the upgrade if the buffer contents have been modified after the hash
was enshrined. This prevents any party from sneaking unauthorized changes
into a core program upgrade between preparation and activation.

## Backwards Compatibility

This proposal does not introduce any breaking changes. The upgrade mechanism
exists off of the runtime's "hot path" until a feature gate is activated for
a specific upgrade.

When an upgrade is conducted, details pertaining to the upgrade's backwards
compatibility will be provided.
