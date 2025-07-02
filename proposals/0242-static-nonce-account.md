---
simd: '0242'
title: Static Nonce Account Only
authors:
  - Andrew Fitzgerald (Anza)
category: Standard
type: Core
status: Implemented
created: 2025-02-03
feature: 7VVhpg5oAjAmnmz1zCcSHb2Z9ecZB2FQqpnEwReka9Zm (https://github.com/anza-xyz/agave/issues/6386)
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
- However, that relaxation cannot happen without specifying the interaction
  with nonce accounts. Rather than complicating the protocol and specifying the
  interactions between lookups and nonces, this proposal aims to simplify by
  restricting nonce accounts to statically declared accounts.

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
- statically included account
  - An account that has its' `Pubkey` directly included in the transaction
    message.

## Detailed Design

- Nonce transactions MUST have a nonce account index that is less the number of
  statically included accounts in the transaction.
- Leader nodes MUST drop nonce transactions that do not meet this requirement
  without including them in a block.
- Nonce transactions that do not meet this requirement are invalid and MUST
  result in the entire block being rejected by the validators.

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
