---
simd: '0159'
title: Relax Precompile Failure Constraint
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Activated
created: 2024-07-25
feature: 9ypxGLzkMxi89eDerRKXWDXe44UY2z4hBig4mDhNq5Dp (https://github.com/anza-xyz/agave/issues/3245)
---

## Summary

The Solana protocol currently prohibits block producers from including
transactions with precompile instructions that fail signature verification.
This constraint is not ideal since block producers may have to do work to verify
precompiles without a guarantee that they can receive a fee from the
transaction.

## Motivation

Ensure that block producers are sufficiently incentivized to include transactions
which make use of precompile instructions by allowing such transactions to be
included in a block even if verification fails.

## Alternatives Considered

Also considered relaxing the constraint of having valid non-fee payer
transaction-level signatures but those signatures are not signed by the fee
payer so they could be altered in-flight.

## New Terminology

- Precompile instruction-level signature: 
    A signature specified inside a precompile instruction which must be verified
    successfully for the transaction to be executed successfully

## Detailed Design

The Solana protocol will no longer reject blocks with transactions that have
failed precompile verification. Such transactions will be allowed to be
recorded in a block and signature verification failures should be handled
similarly to how SVM program errors are handled. The transaction will be
committed, fees deducted, but no other account state changes should be
persisted.

### Transaction Errors

While transaction errors are not required to be consistent for clients reaching
consensus, they should follow the following specification to give consistent
responses to downstream users and clients.

Precompile verification errors should now be mapped to
`InstructionError::Custom(u32)` such that each `PrecompileError` variant below
maps to a custom error code as annotated below:

```rust
pub enum PrecompileError {
    InvalidPublicKey,           // 0u32
    InvalidRecoveryId,          // 1u32
    InvalidSignature,           // 2u32
    InvalidDataOffsets,         // 3u32
    InvalidInstructionDataSize, // 4u32
}
```

## Impact

End users that submit transactions with invalid precompile instructions will now
be able to see clearly in explorers and other tooling that their transactions
failed rather than those transactions being dropped. They will also be charged
fees for such transactions.

Block producers can be assured that compute time spent on verifying any non-fee
payer signatures will be compensated with fees.

## Security Considerations

NA

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

This change will require the use of a new feature gate which will remove the
precompile failure constraint from produced blocks.
