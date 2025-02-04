---
simd: '0242'
title: Static Nonce Account Only
authors:
  - Andrew Fitzgerald (Anza)
category: Standard
type: Core
status: Review
created: 2025-02-03
feature:
---

## Summary

The advance nonce instruction in a transaction can currently specify any
account in the transaction as the account to advance.
This proposal would restrict the advance nonce instruction to only be able to
advance a statically included account.

## Motivation

- There is a separate proposal to relax the constraints on transaction account
  resolution. That proposal would allow transactions with invalid lookups or
  duplicate accounts to be included in a block, pay fees, but not processed.
- However, that relaxation cannot happen if the nonce account is advanced in a
  looked up account.

## New Terminology

No new terminology is introduced.
Clarification of existing terminology is provided:

- nonce transaction
  - A transaction that includes `SystemInstruction::AdvanceNonceAccount` as its
    first instruction.
- nonce account
  - The account specified in the first account of the
    `SystemInstruction::AdvanceNonceAccount` instruction in a nonce
    transaction.
- recent blockhashes sysvar account
  - The account specified in the second account of the
    `SystemInstruction::AdvanceNonceAccount` instruction in a nonce
    transaction.
- statically included account
  - An account that has its' `Pubkey` directly included in the transaction
    message.

## Detailed Design

- Nonce transactions MUST have a nonce account index that is less than the
  number of statically included accounts in the transaction.
- Nonce transactions MUST have a recent blockhashes sysvar account index that
  is less than the number of statically included accounts in the transaction.
- Leader nodes MUST drop nonce transactions that do not meet this requirement
  without including them in a block.
- Nonce transactions that do not meet this requirement are invalid and MUST
  result in the entire block being rejected by the validators.

For advance nonce processing, all but the first two accounts in the
`SystemInstruction::AdvanceNonceAccount` instruction are ignored, regardless
of whether they are statically are dynamically resolved.

## Alternatives Considered

- Do nothing
  - This means we cannot relax account resolution constraints
- Delete nonces entirely
  - preferred by some, but is a separate proposal and a more thought-out
    transition plan is necessary.

## Impact

- Clients who currently send nonce-transactions with looked up nonce accounts
  will need to change their behavior.

## Security Considerations

- Requires a feature-gate to be enabled to avoid forking the network.

## Backwards Compatibility

- Some previously valid transactions may not be valid under the new
  restrictions.
