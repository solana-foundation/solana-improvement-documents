---
simd: '0297'
title: Relax Invalid Nonced Transactions Constraint
authors: Tao Zhu (Anza)
category: Standard
type: Core
status: Review
created: 2025-06-05
feature:
---

## Summary

This proposal relaxes the handling of invalid durable nonce transactions during
block replay. Instead of rejecting the entire block when an invalid nonce is
encountered, the transaction should be marked as failed, skipped for state
modifications, but still committed to the block without charging a transaction
fee.

## Motivation

The current consensus behavior specifies that invalid durable nonce transactions
should result in block failure during replay. This behavior hinders forward
compatibility with asynchronous block execution. The goal is to align invalid
nonce handling with the treatment of other soft transaction failures (e.g.,
relax fee payer, relax ALT, etc).

## New Terminology

None

## Detailed Design

### Current Behavior

A transaction using a durable nonce fails block replay (causing the entire
block to be rejected) if any of the following occurs:

- The nonce account is not a statically included account.
- The nonce account does not exist.
- The nonce account is not properly initialized.
- The stored nonce does not match the transaction's recent blockhash.
- The transaction fails to advance the nonce.


### Proposed Change

Update replay logic to handle invalid nonce transactions differently:

1. For failure that can be checked without accessing account state, namely:

   - The nonce account is not a statically included account.

Replay logic remains unchanged - entire block is rejected;

2. For failures require account state to verify, namely: 

   - The nonce account does not exist.
   - The nonce account is not properly initialized.
   - The stored nonce does not match the transaction's recent blockhash.
   - The transaction fails to advance the nonce.

Replay treats these transactions as non-state-modifying and non-fee-charging
failures, as follows:

   - The transaction is not executed.
   - The transaction is not included in status cache.
   - The transaction’s non-execution CU cost (i.e., transaction's static CUs,
     plus actual CUs for loading accounts) still applies to the block limit.
   - The transaction is recorded in the block (marked as failed).
   - No account state is modified, including the nonce account (i.e., nonce is
     not advanced) and fee payer account (it is not charged with the
     transaction's fee).
   - The block is not rejected as long as all other transactions replay
     successfully.


## Alternatives Considered

N/A

## Impact

- Invalid nonce transactions may be included in multiple blocks but will not be
  charged fees.

- The inclusion of the same nonce transaction in multiple blocks may affect RPC
  behavior, such as querying a transaction by signature. The exact handling of
  this is outside the scope of this proposal.

## Security Considerations

The relaxed model still prevents nonce reuse and enforces single-use semantics,
as the transaction does not advance the nonce if it fails. There is no impact
on replay safety, and the ledger remains consistent across nodes.

## Backward Compatibility

This change is **not backward compatible** with current validator behavior. It
must be activated via a feature gate and coordinated with a network upgrade.
