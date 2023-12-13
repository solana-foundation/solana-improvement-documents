---
simd: '0094'
title: Deprecate `executable` update in bpf loader
authors:
  - Haoran Yi
category: Standard
type: Core
status: Draft
created: 2023-12-13
feature: https://github.com/solana-labs/solana/issues/33970
---

## Summary

Add a new feature to deprecate executable account metadata update during bpf
program deployment.

## Motivation

We want to deprecate the usage of *executable* metadata on account for program
runtime. The new variant of Bpf loader (i.e. V3, V4 etc.) no longer requires
*executable* metadata. However, during the program deployment, bpf loader still
updates *executable* account metadata, which is not necessary.

Therefore, as we are migrating to the new Bpf loader, we are going to add a
feature to deprecate executable account metadata update during bpf program
deployment, so that we can activate the feature and deprecate *executable*
metadata in program runtime for the new kinds of Bpf loaders.


## Alternatives Considered

None

## New Terminology

None

## Detailed Design

When the feature - "deprecate executable account metadata update" is activated,
the bpf loader will no longer update *executable* metadata to true after
program deployment.

The PR for this work is at https://github.com/solana-labs/solana/pull/34194

## Impact

1. People should no longer reply on *executable* metadata on the account. Instead,
   they can use an exported *fn is_executable(&account)* from solana sdk.


## Security Considerations

Because when the feature is activated, we should have already migrated to Bpf
loader V3. Bpf loader V2 program deployment should have already been disabled
(https://github.com/solana-foundation/solana-improvement-documents/pull/93).

For new kinds of bpf loaders, the account's owner and bpf loader metadata stored
in the account's data together should correctly tell whether the account is
executable.

A small concern is that we may miss places in the code that still relies on
account *executable*. However, with good testing, we should be able to cover all
of them. And a fix for this should not be hard.

## Backwards Compatibility

Incompatible.
