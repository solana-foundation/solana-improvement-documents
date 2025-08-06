---
simd: '0249'
title: Delay Commission Updates
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2025-02-18
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Allow validators to update their commission rate at any time but delay those
commission updates for at least one full epoch.

## Motivation

Validators should be able update their desired commission at any time. That
said, stake delegators should always have ample time to re-delegate their stake
in response to validator commission rate changes.

## New Terminology

NA

## Detailed Design

### Runtime

For a given epoch `E`, the earned inflation rewards for each vote account are
calculated at the beginning of the next epoch `E + 1`. During inflation reward
calculation, use the inflation rewards commission rate from the vote account
state as it existed at the beginning of epoch `E - 1`. If the vote account did
not exist at that time, fall back to the vote account state at the beginning of
epoch `E`. And if the vote account did not exist then either, fall back
to the vote account state at the beginning of epoch `E + 1`.

#### UpdateCommission

Update the core vote program to no longer restrict commission increases from
happening during the first half of the epoch.

## Alternatives Considered

NA

## Impact

- Validators will need to wait at least one full epoch before their commission
updates are applied.

- Stake delegators will have at least one full epoch to react to commission
updates.

## Security Considerations

NA

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

A feature gate will be used to simultaneously update vote program rules around
commission rate updates as well as update the runtime's commission calculations
at epoch boundaries.