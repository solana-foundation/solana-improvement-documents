---
simd: '0319'
title: Remove Accounts `is_executable` Flag Entirely
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Review
created: 2025-03-18
feature: TBD
extends: 0162
---

## Summary

Remove the accounts `is_executable` flag from the protocol entirely.

## Motivation

See SIMD-0162 for the reasons why the `is_executable` flag is unnecessary
protocol complexity. That SIMD however only removes all checks of the flag
that can abort a transaction. There are other influences the flag has on
consensus, which shall be removed as well.

## New Terminology

None.

## Detailed Design

The following changes in consensus relevant behavior must occur with the
activation of the feature TBD:

### Account storage file format

When loading accounts the flag must be ignored. When storing accounts the
flag should be treated as being always `false`.

### Snapshot minimization special case

Program data accounts (owned by loader-v3) must not be filtered by their
`is_executable` flag anymore.

### Account hash

The flag must not be added to the input of the hash function anymore.
Note that this is different from hashing it as always `false`.

### VM serialization

ABI v2 will simply not have the flag from the start, however ABI v0 and v1 must
change their serialization of the flag to be `0u8`.

### RPC and Geyser

Similarly, all other external interfaces must also always return `false` for
the `is_executable` flag.

### Loader Management Instructions

(Re)deployments in the loader programs must stop setting the `is_executable`
flag.

### CPI special case

Currently CPI ignores changes made by the caller to instruction accounts which
have the flag set, meaning even requesting write access to a program account
throws no error. Instead the flag must now be ignored, meaning all changes made
by the caller to instruction accounts are treated equally, regardless of the
`is_executable` flag.

Because programs did not have to supply `AccountInfo`s for accounts with the
`is_executable` flag set so far, they would be missing and abort the
transaction. Thus, in case a `AccountInfo` is missing, it must not throw
`InstructionError::MissingAccount` anymore but instead not try to update the
account payload length field (as that is missing if there is no `AccountInfo`).

CU consumption must remain unchanged, that is: CUs will continue to be charged
either way.

## Alternatives Considered

None.

## Impact

The changes to the snapshots and account hashes should be irrelevant. The
changes to the VM serialization should be mostly identical to the existing
behavior. The changes to the CPI special case will technically allow for a new
failure mode, when a caller attempts to give write access to a program
account to a callee, but this case does not seem to occur in currently deployed
dApps.

## Security Considerations

None.
