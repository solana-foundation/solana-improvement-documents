---
simd: "0085"
title: Additional Fee Collector Constraints
authors:
  - Justin Starry
category: Standard
type: Core
status: Draft
created: 2023-11-05
feature: https://github.com/solana-labs/solana/issues/33888
---

## Summary

Burns fees if they would otherwise be collected into an account that violates
one of the following constraints:

1. Must be a system owned account
2. Must be rent-exempt

This applies to both transaction fees and rent fees. Note that the "rent-exempt"
constraint was already added for rent fee collection in
[Feature 30151](https://github.com/solana-labs/solana/issues/30151).

## Motivation

1. Since fee collection occurs outside of the runtime, it's generally a good
   idea to reduce the number of account modification edge cases.
2. Prevent new rent paying accounts from being created

## Alternatives Considered

NA

## New Terminology

NA

## Detailed Design

At the end of a block, validators MUST NOT distribute fees to accounts that are
not system owned and/or rent-exempt. Instead, they MUST burn the fees by not
distributing them to anyone.

## Impact

New and existing validators must ensure that their fee collection account is
rent-exempt and owned by the system program in order to receive fees. Since the
Solana Labs validator implementation currently requires the fee collector
account to be same account as the fee payer for vote transactions, this is
unlikely to impact any validators unless they run a custom implementation.

Validators will still be able to collect fees into durable nonce accounts if
they wish. If a validator does not wish to use a hot wallet to have custody
over collected fees, they may use durable nonce accounts which have a
configurable authority address.

## Security Considerations

Note that durable nonce accounts are system owned and rent exempt and can
therefore continue to be used for fee collection.
