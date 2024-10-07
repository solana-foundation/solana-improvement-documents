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

1. Emulate account "executable" from owner and account's data. It turns out the
   emulate is very expensive. It increases "serialization" time by more than 10X.

## New Terminology

None

## Detailed Design

When the feature - "deprecate executable update" is activated, the bpf loader-V3
will no longer update *executable* flag to true after program deployment.

The "executable" flag remains false after program deployment. In future, we may
consider not to serialize account executable any more.

Another change is that CPI won't ignore changes made by caller to instruction
accounts. Currently, during CPI call, if the callee account's executable is set,
the existing instruction account is used without going through the translation.
With this change, there is no special shortcut for the instruction accounts.
They will all go through the general translation code path. Since it is a more
general code path, we are guaranteed to be correct. The slightly downside is
that it become less efficient in theory. But we don't think there is going to
noticeable differences.

## Impact

This will affect the following scenarios.

- program accounts hash after deployment. Since this change is guarded by a
  feature, program accounts hash change won't break consensus.
- any dapps that depend on `is_executable` to be true on the serialized
  instruction accounts. Existing program deployed on chain will work fine.
  However, if the program is redeployed, it may be broken. Before redeployment,
  dapps developer will need to check and update the program if it depends on
  `is_executable`.

## Security Considerations

None

## Backwards Compatibility

Incompatible.
