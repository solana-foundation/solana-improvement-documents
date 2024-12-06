---
simd: "0208"
title: Allow Closed Programs to be Recreated
authors:
  - keith@metaplex.foundation
  - justin.starry@icloud.com
category: Standard
type: Core
status: Draft
created: 2024-12-06
feature:
---

## Summary

This proposal is the migration of the discussion found at
https://github.com/solana-labs/solana/pull/27460, regarding closed program
accounts not being recoverable. This SIMD proposes adding in the ability to
reopen a closed program through the same mechanism as standard program deploys.

## Motivation

As it is the default behavior of all Solana accounts to be reopenable, the lack
of this ability for program accounts seems like an oversight. More importantly,
because it does not follow the default behavior, it would not be obvious to the
average program deployer that closing a program is an irreversible operation
that could permanently brick any accounts owned by the closed program.

## Alternatives Considered

## New Terminology

## Detailed Design

## Impact

- Previously closed programs could be reopened and accounts recovered.

## Security Considerations

- Intentionally closed programs expecting the behavior that programs are
  incapable of being reopened.
