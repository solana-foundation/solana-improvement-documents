---
simd: '0290'
title: Relax Fee Payer Constraint
authors:
  - Andrew Fitzgerald (Anza)
category: Standard
type: Core
status: Review
created: 2025-05-29
feature:
---

## Summary

This proposal aims to relax the handling of invalid fee payers in Solana.
Currently, if a transaction with an invalid fee payer is included in a block,
the entire block is rejected.
This proposal suggests that instead of rejecting the entire block, the
transaction with the invalid fee payer should simply skip execution.

## Motivation

The current constraint forces block-validation to be synchronous since in order
to determine if a block is valid or not, some subset of transactions must be
executed in order to determine if the fee payer has sufficient funds to pay the
transaction fees.
By relaxing this constraint, we move one step closer towards asynchronous
validation/execution.

## New Terminology

None.

## Detailed Design

A transaction has a statically determined fee `fee` lamports.
A transaction can successfully pay fees if:

- The fee payer account has exactly `fee` lamports.
- The fee payer account has at least X + `rent_exempt_reserve` lamports.

If the fee payer account does not meet either of these conditions, the transaction
may be included in a block, but it must not be executed.
The transaction will have no effect on the state.

Invalid fee payer transactions will count their requested,
or default, compute units (CUs) towards block limits.
This is intended to make it strictly cheaper to process invalid fee payer
transactions compared to valid fee payer transactions of the same construction.

## Alternatives Considered

- Requiring some sort of fee-payer lock up/bonding mechanism to ensure that fee
  payers have sufficient funds to pay for the transaction fees.
  - This is more complex compared to this proposal.

## Impact

Transactions that are unable to pay fees may be included in blocks.

## Security Considerations

- Possible attack vector where a malicious leader can spam transactions with
  invalid fee payers. Mitigation for this is charging full CUs for these.

## Drawbacks

- If there is no interest in simplifying block-validation to allow for
  asynchronous, this proposal is not necesary.
- Concern about data propagation for without paying fees to the network (burn):
  - This concern has been raised in the past when this has been discussed.
  - However, the concern is largely invalid since even without this proposal,
    a malicious leader could still propagate data through the network for free
    by simply using an invalid fee payer.

## Backwards Compatibility

- All previously valid transactions and blocks are still valid.
- Blocks produced after the change may be rejected by previous versions of the
  validator client(s).
