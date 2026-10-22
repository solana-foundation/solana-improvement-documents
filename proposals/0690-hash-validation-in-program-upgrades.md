---
simd: '0690'
title: Hash validation in program upgrades
authors:
  - Joe Caulfield (Anza)
  - febo (Anza)
category: Standard
type: Core
status: Review
created: 2026-10-07
feature: (fill in with the feature key and github tracking issues once accepted)
extends: SIMD-0088, SIMD-0418 and SIMD-0495
---

## Summary

This proposal adds a hash validation step to native and Loader v2 to v3
program migrations and Core BPF program upgrades.

## Motivation

Program migrations and upgrades via a feature gate are extremely sensitive
processes that must be handled carefully. Although a migration or upgrade can
only be activated via a feature gate, there is currently no validation to ensure
that the contents of the buffer account match the expected program data.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0088]: Enable Core BPF Programs**

    Defines the procedure for migrating native programs to Core BPF.

[SIMD-0088]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0088-enable-core-bpf-programs.md

- **[SIMD-0418]: Enable Loader v2 to v3 Program Migrations**

    Defines the procedure for migrating Loader v2
    programs to Loader v3.

[SIMD-0418]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0418-enable-loader-v2-to-v3-program-migrations.md

- **[SIMD-0495]: Enable Core BPF Upgrade Procedure**

    Defines the procedure for upgrading Core BPF programs.

[SIMD-0495]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0495-core-bpf-upgrade-procedure.md

## New Terminology

N/A

## Detailed Design

The procedures for migrating native programs to Core BPF (SIMD-0088), migrating
Loader v2 programs to Loader v3 (SIMD-0418), and upgrading Core BPF programs
(SIMD-0495) involve creating a buffer account with the contents of the program.
The runtime then replaces the existing native, Loader v2, or Loader v3 program
account with new Loader v3 program and program data accounts.

This proposal adds a hash validation step to the migration and upgrade
procedures. Before migrating or upgrading a program, the runtime needs to
validate that the hash of the program data (ELF bytes) in the buffer account
matches the expected hash. The expected hash is the SHA-256 hash of the ELF bytes
and must be supplied alongside the feature gate in accordance with the validator
implementation.

When the feature activates, the runtime must validate the hash of the buffer
contents after verifying that the buffer account exists, is owned by the
Upgradeable Loader, and deserializes as `UpgradeableLoaderState::Buffer`:

1. Extract the ELF bytes from the source buffer account, excluding trailing zero
   bytes.
2. Compute the SHA-256 hash of the extracted ELF bytes.
3. Compare the computed hash with the expected hash provided in the feature
   gate.

If the hashes match, the remaining checks and steps are unchanged from their
respective proposals:

- Native program migrations to Core BPF: [SIMD-0088].
- Loader v2 program migrations to Loader v3: [SIMD-0418].
- Core BPF program upgrades: [SIMD-0495].

If the hashes do not match, the migration fails without modifying the program.
In this case, the feature is still marked as activated, but the program data
remains unchanged.

Hash validation ties the migration and upgrade to the expected program data,
rather than relying solely on the source buffer account's address.

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                                    |
|---------------------------------|-------------------------------------------|
| Transaction Execution (Runtime) | Validates the hash of the buffer contents |
| Virtual Machine                 |                                           |
| Block Packing                   |                                           |
| Consensus                       |                                           |
| Gossip                          |                                           |
| Turbine                         |                                           |
| Snapshots                       |                                           |
| On-Chain Core BPF Programs      |                                           |
| Other (please describe)         |                                           |

## Alternatives Considered

Continue with the migration procedure without hash validation. This approach is
not recommended due to the risk of migrating incorrect program data, which could
lead to unexpected behavior or security vulnerabilities in important programs.

## Impact

The runtime will only migrate a program if the hash of the source buffer's ELF
bytes matches the expected hash. A hash mismatch aborts the migration and leaves
the existing program unchanged.

## Security Considerations

Migrating native and Loader v2 programs, or upgrading Core BPF programs, replaces
essential programs with the contents of another account. Hash validation protects
against accidental or malicious substitution of incorrect program data, including
changes to the buffer's ELF bytes between feature gate creation and activation.

## Backwards Compatibility

This proposal does not introduce any breaking changes, and the upgrade mechanism
is not part of the runtime's transaction processing. It is only executed when a
feature gate is activated for a specific program migration.
