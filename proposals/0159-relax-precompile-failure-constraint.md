---
simd: '0159'
title: Relax Precompile Failure Constraint
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Draft
created: 2024-07-25
feature: (fill in with feature tracking issues once accepted)
---

## Summary

The Solana protocol currently prohibits block producers from including transactions
with precompile instructions that fail verification. This constraint is not ideal
since block producers have to do work to verify precompiles without a guarantee
that they can receive a fee from the transaction.

## Motivation

Ensure that block producers are sufficiently incentivized to include transactions
which make use of precompile instructions by allowing such transactions to be
included in a block even if verification fails.

## Alternatives Considered

NA

## New Terminology

NA

## Detailed Design

The Solana protocol will no longer reject blocks with transactions that have
failed precompile verification. Such transactions will be allowed to be recorded
in a block and verification failures should be handled similarly to how SVM
program instruction failures are handled. The transaction will be included, fees
deducted, but no other account state changes should be persisted.

## Impact

End users that submit transactions with invalid precompile instructions will now
be able to see clearly in explorers and other tooling that their transactions
failed rather than those transactions being dropped. They will also be charged
fees for such transactions.

Block producers can be assured that compute time spent on verifying precompiles
will be compensated with fees.

## Security Considerations

NA

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

This change will require the use of a new feature gate which will remove
the precompile failure constraint from produced blocks.
