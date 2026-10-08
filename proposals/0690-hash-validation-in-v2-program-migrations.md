---
simd: '0690'
title: Hash validation in v2 program migrations
authors:
  - febo (Anza)
category: Standard
type: Core
status: Review
created: 2026-10-07
feature: (fill in with the feature key and github tracking issues once accepted)
extends: SIMD-0418
---

## Summary

This proposal adds a hash validation step to the Loader v2 to v3 program
migration procedure described in SIMD-0418.

## Motivation

Program migration via a feature gate is an extremely sensitive process that must
be handled carefully. Although a migration can only be activated via a feature
gate, there is currently no validation to ensure that the contents of the buffer
account match the expected program data.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0418]: Enable Loader v2 to v3 Program Migrations**

    Defines the procedure for migrating Loader v2
    programs to Loader v3.

[SIMD-0418]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0418-enable-loader-v2-to-v3-program-migrations.md

## New Terminology

N/A

## Detailed Design

The migration procedure (SIMD-0418) for Loader v2 programs to Loader v3 involves
creating a buffer account with the contents of the program. The runtime then
replaces the existing Loader v2 program account with new Loader v3 program and
program data accounts.

This proposal adds a hash validation step to the migration procedure. Before
migrating a program, the runtime needs to validate that the hash of the program
data (ELF bytes) in the buffer account matches the expected hash. The expected
hash is the SHA-256 hash of the ELF bytes and must be supplied alongside the
feature gate in accordance with the validator implementation.

When the feature activates, the runtime must:

1. Extract the ELF bytes from the source buffer account, excluding trailing zero
   bytes.
2. Compute the SHA-256 hash of the extracted ELF bytes.
3. Compare the computed hash with the expected hash provided in the feature
   gate.

If the hashes match, the migration follows the steps described in SIMD-0418
&mdash; these steps are unchanged.
If the hashes do not match, the migration fails without modifying the program.

Hash validation ties the migration to the expected program data, rather than
relying solely on the source buffer account's address.

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

Migrating Loader v2 programs replaces essential programs with the contents of
another account. Hash validation protects against accidental or malicious
substitution of incorrect program data, including changes to the buffer's ELF
bytes between feature gate creation and activation.

## Backwards Compatibility

This proposal does not introduce any breaking changes, and the upgrade mechanism
is not part of the runtime's transaction processing. It is only executed when a
feature gate is activated for a specific program migration.
