---
simd: '0231'
title: Disable Account Rent Epoch Updates
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2025-01-23
supersedes: https://github.com/solana-foundation/solana-improvement-documents/pull/175
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Rent fee collection has been disabled in SIMD-0084 but that proposal did not
disable account rent epoch updates. In order to fully disable rent collection,
this SIMD proposes disabling all rent epoch field updates.

## Motivation

Since the rent epoch field is no longer necessary for tracking when accounts are
rent exempt or last had rent collected, we no longer need to keep this field
updated. By not updating this field, validators no longer need to waste time
scanning accounts for partitioned rent collection.

## New Terminology

NA

## Detailed Design

New accounts MUST no longer have their rent epoch field updated to `u64::MAX`
upon creation. The default value for this field in new accounts will be `0`.
This includes new builtin accounts like new sysvars and precompiles which should
already have their initial rent epoch field set to `0`.

Existing accounts MUST not have their rent epoch field updated when loaded as
writable by an executable transaction OR during partitioned rent collection.

Since existing accounts will no longer have their rent epoch field update
updated, partioned rent collection will no longer have any effect and can be
safely disabled and removed.

## Alternatives Considered

NA

## Impact

Improved block processing performance by removing the need to load all accounts
once per epoch for partitioned rent collection. Note that the epoch accounts
hash calculation similarly loads all accounts once per epoch but serves the
important role of ensuring that all validators have the same set of account
state.

After this proposal has been applied and activated, new accounts will be
distinguishable from old accounts because they will have a different rent epoch
field. This will be observable both onchain and offchain until the rent epoch
field is fully deprecated in a future proposal.

## Security Considerations

Rent related code changes often come along with a lot of edge cases to consider
so changes should be made carefully to avoid introducing any bugs.

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

Changes require feature gates for activation to avoid any backwards incompatiblity
