---
simd: '0094'
title: Deprecate `executable` update in bpf loader v3
authors:
  - Haoran Yi
category: Standard
type: Core
status: Draft
created: 2023-12-13
feature: https://github.com/solana-labs/solana/issues/33970
extends: 0162
---

## Summary

Deprecate executable account metadata update during bpf program deployment.

## Motivation

Currently, when a program is deployed, the "executable" flag on the account is
set for legacy reasons. Now (a) old bpf loaders, such as v1 and v2, are disabled
on mainnet (SIMD-0093). (b) We are going to remove accounts executable checks
for runtime execution (SIMD-0162). (c) The future bpf loader-v4 no longer need
"executable" flag.

Because of these reasons, we want to deprecate executable account meta data
update during bpf program deployment for loader-v3.

## Alternatives Considered

None

## New Terminology

None

## Detailed Design

When the feature - "deprecate executable account metadata update" is activated,
the bpf loader v3 will no longer update *executable* metadata to true after
program deployment.

The "executable" flag will remain false after program deployment.

## Impact

This will affect the following scenarios.

- program accounts hash after deployment
- any dapps that depend on `is_executable` to be true on serialized instruction accounts
- CPI computation won't ignore changes made by caller to instruction accounts
  (Currently CPI ignores changes made by the caller to instruction accounts
  which has the `is_executable` flag set)

## Security Considerations

None

## Backwards Compatibility

Incompatible.
