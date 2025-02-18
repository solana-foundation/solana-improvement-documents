---
simd: 'XXXX'
title: Delay Commission Updates
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2025-02-18
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Allow validators to update their commission values without restriction but delay
those commission changes for at least one full epoch.

## Motivation

Stake delegators should have time to re-delegate their stake in response to
validator commission rate changes.

## New Terminology

NA

## Detailed Design

- During inflation reward distribution for an epoch `E`, use the commission rate
set in the vote account state at the beginning of the epoch `E - 1` when
calculating validator commissions. This is the same vote account state used to
calculate the leader schedule for epoch `E` so it must already be available
in-protocol. The only exception is inflation reward distribution for epoch `E ==
0`. In that case, use the commission rate set by vote accounts in the genesis
config.

- Update the core vote program to no longer restrict commission updates in any way.

## Alternatives Considered

An alternative approach is to track recent commission values over the past few
epochs in the vote account state. This bookkeeping increases state size
requirements of the vote account for each type of commission set by a validator.

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

No backwards compatibility issues. Commission updates will be less restrictive.
