---
simd: '0159'
title: Relax Transaction Signature Constraints
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Draft
created: 2024-07-25
feature: (fill in with feature tracking issues once accepted)
---

## Summary

The Solana protocol currently prohibits block producers from including
transactions with any invalid signature whether that signature be the fee-payer
signature, other transaction-level signatures or precompile instruction-level
signatures. This constraint is not ideal since block producers may have to do
work to verify all of the signatures used in a transaction without a guarantee
that they can receive a fee from the transaction.

## Motivation

Ensure that block producers are sufficiently incentivized to include transactions
which make use of precompile instructions by allowing such transactions to be
included in a block even if verification fails.

## Alternatives Considered

Also considered only relaxing the constraint prohibiting transactions with
precompile instruction-level invalid signatures. But since non-fee payer
transaction-level signatures are handled similarly, it was thought best to relax
the constraint for those as well so that as long as the fee payer signature is
valid, the block producer is able to collect fees for including the transaction.

## New Terminology

- Precompile instruction-level signature: 
    A signature specified inside a precompile instruction which must be verified
    successfully for the transaction to be executed successfully
- Non-fee payer transaction-level signature:
    Any signature included in the top-level signatures field in a transaction
    besides the first listed signature which denotes the fee-payer signature.

## Detailed Design

The Solana protocol will no longer reject blocks with transactions that have
failed precompile verification or have non-fee payer transaction-level
signatures that fail verification.  Such transactions will be allowed to be
recorded in a block and signature verification failures should be handled
similarly to how SVM program errors are handled. The transaction will be
committed, fees deducted, but no other account state changes should be
persisted.

The signature count included in the bank hash calculation should still include
the number of transaction-level signatures for all transactions included in a
block regardless of verification success.

## Impact

End users that submit transactions with invalid precompile instructions or
invalid non-fee payer transaction-level signatures will now be able to see
clearly in explorers and other tooling that their transactions failed rather
than those transactions being dropped. They will also be charged fees for such
transactions.

Block producers can be assured that compute time spent on verifying any non-fee
payer signatures will be compensated with fees.

## Security Considerations

NA

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

This change will require the use of a new feature gate which will remove the
precompile failure constraint and the non-fee payer transaction-level signature
failure constraint from produced blocks.
