---
simd: '0084'
title: Disable rent fees collection
authors:
  - Haoran Yi
category: Standard
type: Core
status: Draft
created: 2023-11-03
feature: https://github.com/solana-labs/solana/issues/33946
---

## Summary

Add a new feature to disable rent fees collections.

## Motivation

The total number of rent paying accounts on solana network is phasing out. And
it is no longer possible to create new rent paying account on the network.
Therefore, when all the rent paying accounts are gone in the network, we would
like to disable rent fee collection on the network through a feature.


## Alternatives Considered

None

## New Terminology

None

## Detailed Design

When the feature - "disable rent fees collections" is activated, rent will no
longer be collected from accounts nor will it be distributed to validators.

Note that this does **not** change the requirement that existing rent-paying
accounts, which need to be made rent-exempt first before any withdrawals can be
make from them.

## Impact

1. Other implementations of validator client will not need to implement rent
   processing.

1. The performance of validators will be better since there is no more rent
   processing.

## Security Considerations

There will be no more rent paying accounts once the feature is activated. It is
already impossible to create any new rent paying accounts on the network, there
should be no security issue. However, if for some reason rent paying accounts
still exist or are created in the network, when the feature is activated, the network
will still work as expected. The only difference is just that no more rents are
collected or distributed.


## Backwards Compatibility

Incompatible.