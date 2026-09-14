---
simd: '0596'
title: Increase TxV1 Account Lock Limit to 96
authors:
  - Andrew Fitzgerald (Anza)
category: Standard
type: Core
status: Draft
created: 2026-08-11
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Increase the maximum number of account addresses in a v1 transaction from 64
to 96. This limit includes all addresses in the transaction's account list:
signers, program IDs, and both writable and read-only accounts. Legacy and
v0 transaction formats retain the 64-account limit.

## Motivation

The runtime limits v1 transactions to 64 account locks. Raising the limit allows
v1 transactions to use more accounts while leaving room for signatures,
instructions, and instruction data within the transaction size limit.

For example, onchain swap routing can require accounts for multiple liquidity
pools and the programs they use. A higher account limit allows a single
transaction to access more pools, enabling more complex routes that can offer
better execution prices.

## New Terminology

None.

## Detailed Design

Once the associated feature gate is activated, v1 transactions MUST be allowed
to lock up to 96 accounts, inclusive. A v1 transaction with more than 96
accounts MUST fail sanitization. During block production, the leader MUST
discard such a transaction. During replay, a validator MUST mark the block dead
if it contains such a transaction.

Legacy and v0 transactions MUST remain limited to 64 account locks.

The `increase_tx_account_lock_limit` feature, which raises the account lock
limit to 128, MUST be removed before this proposal's feature is introduced.
The two features therefore have no precedence or interaction rules.

## Alternatives Considered

V1 transactions have a 4096-byte size limit and encode each account address
directly as 32 bytes, without address lookup tables. A limit of 96 accounts
uses 3072 bytes for addresses, leaving 1024 bytes for signatures, headers,
configuration, instructions, and instruction data. By comparison, 128 account
addresses alone would consume the entire transaction size limit.

The 96-account limit balances access to more accounts with room for the rest
of the transaction. Transactions MUST still fit within the transaction size
limit, so those with more signatures or instruction data may fit fewer than
96 accounts.

Extending the higher account limit to legacy and v0 transactions was also
considered. The goal is to eventually remove support for these formats, so
increasing their capabilities would encourage further use of formats intended for
retirement. Limiting the increase to v1 encourages applications to migrate
to the newer format.

## Impact

Applications can use up to 96 accounts in a v1 transaction. Legacy and v0
transactions are unaffected.

## Security Considerations

Transactions can lock 50% more accounts, increasing their potential scheduling
and account-loading cost. Existing transaction and block resource limits remain
unchanged.

## Backwards Compatibility

The change only relaxes the limit for v1 transactions and is feature gated.
