---
simd: '0232'
title: Custom Fee Collector Account
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2025-01-24
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Allow validators to specify a custom fee collector account where all earned
block fees will be deposited.

## Motivation

Validator fee collector accounts must currently be the same as their identity
hot wallet account. This means that program derived addresses are unable to be
used for fee collection adding friction to validators distributing their revenue
in custom ways. By allowing validators to specify a separate custom fee
collector address, they can use onchain programs to customize how their block
revenue is distributed.

## Alternatives Considered

NA

## New Terminology

NA

## Detailed Design

This proposal requires the adoption of SIMD-0180 and SIMD-0185. SIMD-0180
adjusts the leader schedule algorithm to make it possible to designate a
specific vote account for a given leader slot. SIMD-0185 adds a new fee
collector address field to the vote account state.

After adoption of SIMD-0180 and SIMD-0185, a given block's fee collector address
can be looked up via the designated vote account for the leader schedule slot
that the block was produced in.

In order to eliminate the overhead of tracking the latest fee collector address
of each vote account, the fee collector address should be fetched from the state
of the vote account at the beginning of the previous epoch. This is the same
vote account state used to build the leader schedule for the current epoch.

Note that the fee-collector constraints defined in SIMD-0085 still hold. The
designated fee collector must be a system program owned account that is
rent-exempt after receiving collected block fee rewards. If either of these
constraints is violated, the fees collected for that block will be burned. 

## Impact

Validator identity and fee collector accounts no longer need to be the same
account. This opens up the ability to use PDA accounts for fee collection.

## Security Considerations

NA

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

This change will require the use of a new feature gate which will enable
collecting fees into custom fee collector addresses if specified by a validator.
