---
simd: '0084'
title: Disable rent fees collection 
authors:
  - Haoran Yi   
category: Standard
type: Core
status: Draft
created: 2023-11-03
feature: (fill in with feature tracking issues once accepted)
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

When the feature - "disable rent fees collections" is activated, all accounts
will be treated as "Rent Exempt".

The PR is at https://github.com/solana-labs/solana/pull/33945

## Impact

None

## Security Considerations

None

## Backwards Compatibility

Not applicable.
