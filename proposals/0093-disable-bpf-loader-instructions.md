---
simd: '0093'
title: Disable Bpf loader V2 program deployment
authors:
  - Haoran Yi
category: Standard
type: Core
status: Draft
created: 2023-12-13
feature: https://github.com/solana-labs/solana/issues/33970
---

## Summary

Add a new feature to disable Bpf loader V2 program deployment.

## Motivation

We want to deprecate the usage of *executable* metadata on accounts for program
runtime. The new variant of Bpf loader (i.e. V3/V4 etc.) no longer requires
*executable* metadata. However, the old Bpf loader (v2) still uses *executable*
metadata during its program deployment. And this is a blocker for deprecating
the usage of *executable* metadata for program runtime. Therefore, as we are
migrating from the old Bpf loader V2 to the new Bpf loader (V3/V4), we are going
to add a feature to disable old V2 Bpf program deployment so that we can
activate the feature and deprecate *executable* metadata in program runtime for
the new kinds of Bpf loaders.


## Alternatives Considered

None

## New Terminology

None

## Detailed Design

When the feature - "disable bpf loader instructions" is activated, no program of
bpf loader V2 can be deployed. Any such deployment attempt will result in a
"UnsupportedProgramId" error.

The PR for this work is at https://github.com/solana-labs/solana/pull/34194

## Impact

1. New programs will no longer be deployable with Bpf loader V2. People will have
   to migrate their program deployment with the new Bpf loader, i.e. V3 or V4.


## Security Considerations

Because when the feature is activated, people should have already migrated their
new programs to Bpf loader V3/V4. And existing already-deployed Bpf loader V2
programs will still run correctly. Hence, there should be no security concerns.

## Backwards Compatibility

Incompatible.
