---
simd: '0084'
title: Disable rent fees collection
authors:
  - Haoran Yi
category: Standard
type: Core
status: Implemented
created: 2023-11-03
feature: [CJzY83ggJHqPGDq8VisV3U91jDJLuEaALZooBrXtnnLU](https://github.com/solana-labs/solana/issues/33946)\
development:
  - Anza - [Implemented](https://github.com/solana-labs/solana/pull/33945)
  - Firedancer - Implemented
---

## Summary

Add a new feature to disable rent fees collections.

## Motivation

On solana network, (Rent) [https://solana.com/docs/intro/rent] was introduced to
account for the storage costs of an account initially. However, as the disks
become cheaper and cheaper. The cost of storing the account becomes negligible.

Furthermore, today it is no longer possible to create any new rent-paying
accounts on the network. Any attempt to create a rent-paying accounts will
result a transaction error. The total number of rent-paying accounts on solana
network is phasing out. Therefore, when all the rent-paying accounts are gone in
the network, we would like to disable rent fee collection on the network
completely through a feature.


## Alternatives Considered

None

## New Terminology

None

## Detailed Design

When the feature "disable rent fees collections" is activated, rent will no
longer be collected from accounts, nor will the collected rent fees be
distributed to validators.

Note that this does **not** change the requirement that existing rent-paying
accounts need to be made rent-exempt first, before any withdrawals can be
made, should there be any such rent-paying accounts.

## Impact

The main impact of this SIMD is to simply the validator design. Rent collection
code is a non-trivial piece of code inside the bank. By removing it, we simply
the overall code of bank, and we no longer need to maintain it in existing
validator client code. This will also help to simplify the development of other
validator client implementations, which don't need to replicate the rent
collection logic.


## Security Considerations

There will be no more rent-paying accounts before this feature is activated. It
is already impossible to create any new rent-paying accounts on the network,
there should be no security issues. However, if for some reason rent-paying
accounts still exist or are created in the network, when the feature is
activated, the network will still work as expected. The only difference is that
rent fees will no longer be collected or distributed.


## Backwards Compatibility

Incompatible.
