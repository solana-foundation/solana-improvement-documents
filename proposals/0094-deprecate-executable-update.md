---
simd: '0094'
title: Deprecate executable update in bpf loader
authors:
  - Haoran Yi
category: Standard
type: Core
status: Draft
created: 2023-12-13
feature: (fill in with feature tracking issues once accepted)
extends: 0162
---

## Summary

Deprecate executable update during bpf program deployment.

## Motivation

Currently, when a program is deployed, the "executable" flag on the account is
set to true for legacy reasons. However, (a) old bpf loaders, such as v1 and v2,
are disabled on mainnet (SIMD-0093); (b) we are going to remove accounts
executable checks for runtime execution (SIMD-0162). (c) the future bpf
loader-v4 no longer need "executable" flag.

Because of these above reasons, "executable" flag becomes irrelevant and
provides no functional benefit for runtime execution. Therefore, we want to
deprecate its update during bpf program deployment for loader-v3.

## Alternatives Considered

None

## New Terminology

None

## Detailed Design

When the feature - "deprecate executable update" is activated, the bpf loader-V3
will no longer update *executable* flag to true after program deployment.

The "executable" flag remains false after program deployment.

## Impact

This will affect the following scenarios.

- program accounts hash after deployment. Since this change is guarded by a
  feature, program accounts hash change won't break consensus.
- any dapps that depend on `is_executable` to be true on the serialized
  instruction accounts. Existing program deployed on chain will work fine.
  However, if the program is redeployed, it may be broken. Before redeployment,
  dapps developer will need to check and update the program if it depends on
  `is_executable`.
- CPI won't ignore changes made by caller to instruction accounts (Currently CPI
  ignores changes made by the caller to instruction accounts which has the
  `is_executable` flag set). For correctness, this change will be fine. It just
  becomes a bit less efficient with more checks.

## Security Considerations

None

## Backwards Compatibility

Incompatible.
