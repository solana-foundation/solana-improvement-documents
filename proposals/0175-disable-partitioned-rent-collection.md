---
simd: '0175'
title: Disable Partitioned Rent Updates
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Accepted
created: 2024-09-25
feature: 2B2SBNbUcr438LtGXNcJNBP2GBSxjx81F945SdSkUSfC
---

## Summary

Partitioned rent collection should be removed since it will no longer be useful
after rent collection is disabled by
https://github.com/solana-labs/solana/issues/33946
(https://github.com/solana-foundation/solana-improvement-documents/pull/84) and
account rewrites are disabled by
https://github.com/solana-labs/solana/issues/26599.

## Motivation

Since partitioned rent collection is no longer necessary for collecting rent or
updating the account rent epoch field, it should be removed to speed up block
production and block replay.

## New Terminology

NA

## Detailed Design

Disabling partitioned rent collection is very straightforward. Partitioned rent
collection is initiated during bank freezing and can simply be not performed if
a feature gate is activated.

## Alternatives Considered

NA

## Impact

Improved block processing performance by removing the need to load all accounts
once per epoch for no good reason. Note that the epoch accounts hash calculation
similarly loads all accounts once per epoch but serves the important role of
ensuring that all validators have the same set of account state.

## Security Considerations

Rent related code changes often come along with a lot of edge cases to consider
so changes should be made carefully to avoid introducing any bugs.

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

Changes require feature gates for activation to avoid any backwards incompatiblity
