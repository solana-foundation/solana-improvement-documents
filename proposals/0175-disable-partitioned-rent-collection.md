---
simd: '0175'
title: Disable Partitioned Rent Updates
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Accepted
created: 2024-09-25
feature: (fill in with feature tracking issues once accepted)
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

The reason that partitioned rent collection was not entirely disabled by earlier
features is because it was desired that any pre-existing rent-paying account
that becomes rent-exempt should have its rent epoch field set to the marker
value of `u64::MAX`. This desired outcome can be achieved in a much more
efficient manner without the need for loading every account once per epoch in
partitioned rent collection.

Disabling partitioned rent collection is very straightforward. Partitioned rent
collection is initiated during bank freezing and can simply be not performed if
a feature gate is activated.

Retaining the behavior of updating the rent epoch field for newly rent exempt
accounts can be done by adjusting the behavior of existing transaction rent
checks. Currently, at the end of transaction processing, each writable account
is checked for rent exemption to ensure that no accounts can be created as
rent-paying. Those checks MUST be modified to additionally set the rent epoch
to the marker value of `u64::MAX` if a pre-existing rent-paying account becomes
rent exempt (note that this can only happen if an account is writable).

Currently, new sysvars, builtins, and precompiles are all created with an
initial rent epoch of `0` rather than the marker value of `u64::MAX`. So this
proposal REQUIRES all new sysvars, builtins, and precompiles to be created with
an initial rent epoch of `u64::MAX` to ensure that they are correctly marked as
rent exempt.

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
