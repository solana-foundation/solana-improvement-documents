---
simd: '0093'
title: Disable Bpf loader V1 program deployment
authors:
  - Haoran Yi
category: Standard
type: Core
status: Draft
created: 2023-12-13
feature: https://github.com/solana-labs/solana/issues/33970
---

## Summary

Add a new feature to disable Bpf loader V1 program deployment.

## Motivation

We want to deprecate the usage of *executable* metadata on account for program
runtime. The new variant of Bpf loader (i.e. V2, V4 etc.) no longer requires
*executable* metadata. However, the old Bpf loader (v1) still use *executable*
metadata during its program deployment. And this is a blocker for deprecating
the usage of *executable* metadata for program runtime. Therefore, as we are
migrating from the old Bpf loader V1 to the new Bpf loader, we are going to add
a feature to disable old Bpf program deployment so that we can activate the
feature and deprecate *executable* metadata in program runtime for the new kind
of Bpf loaders.


## Alternatives Considered

None

## New Terminology

None

## Detailed Design

When the feature - "disable bpf loader instructions" is activated, no program of
bpf loader V1 can be deployed. Any such deployment attempt will result in a
"UnsupportedProgramId" error.

The PR for this work is at https://github.com/solana-labs/solana/pull/34194

## Impact

1. New program will not longer be deployed with Bpf loader V1. People will have
   to migrate their program deployment with the new Bpf loader, i.e. V2.


## Security Considerations

Because when the feature is activated, people should have already migrated their
new programs to Bpf loader V2. And existing already-deployed Bpf loader V1
programs will still run correctly. Hence, there should be no security concerns.

## Backwards Compatibility

Incompatible.
