---
simd: '0418'
title: Enable Loader v2 to v3 Program Migrations
authors:
  - febo (Anza)
category: Standard
type: Core
status: Review
created: 2025-11-27
feature: N/A 
---

## Summary

This proposal introduces a mechanism to migrate Loader v2 programs to Loader
v3.

## Motivation

Loader v2 programs are immutable by design, as the loader does not support
program upgrades. However, several essential Loader v2 programs would benefit
from a feature-gated, consensus-driven mechanism that allows the community to
approve targeted upgrades to specific programs (e.g., SPL Token).

## Alternatives Considered

Previously, Loader v2 programs could be “upgraded” by directly modifying their
account data. This approach is no longer recommended due to the introduction of
the global program cache, since it can cause inconsistencies if account data is
modified in-place.

## New Terminology

N/A

## Detailed Design

Migrating a Loader v2 program to Loader v3 involves creating a buffer
account owned by the Loader v3 (BPF Upgradable Loader) that contains the
ELF bytes of the program’s implementation.

The migration proceeds in the following steps:

* **Create the program data account**:
  A program data account is derived from the program ID. This account must
  not already exist, with one exception: if an account with the derived address
  exists, holds lamports, and is owned by the System Program, it may be reused.
  In that case, any excess lamports are burned. The ELF bytes from the buffer
  account are then copied into this program data account.

* **Rewrite the program account**:
  The existing Loader v2 program account is replaced with a Loader v3 program
  account that references the newly created program data account. Because
  Loader v2 programs are not upgradable, they do not have an upgrade authority,
  and `None` is assigned during migration.

* **Close the buffer account**:
  Once both program and program data accounts have been created and populated,
  the buffer account is closed and any remaining lamports are burned.

In the slot immediately following the feature activation, the program will not be
invocable. This status will last one slot, then the program will be fully operational.
The program ID does not change by this migration process.

Each individual program migration should be controlled by its own feature gate,
which triggers the migration process when activated.

## Impact

With this change, validator clients would no longer be required to implement
changes to essential programs. Instead these programs could be updated just
once. This reduces some engineering overhead on validator teams.

## Security Considerations

Migrating Loader v2 programs have serious security implications surrounding
the replacement of essential programs with the contents of another account.
This is an extremely sensitive process that must be handled with maximum
caution.

## Backwards Compatibility

This proposal itself does not directly introduce any breaking changes. The code
introduced to migrate Loader v2 programs to Loader v3 programs will exist off of
the runtime's "hot path" until it's actually used for a migration.

When a migration is conducted, the Loader v3 version of a program is expected to
be backwards compatible *functionally* &mdash; its Loader v3 version must provide
the exact same results as the Loader v2 program it aims to replace.
