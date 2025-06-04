---
simd: 'XXXX'
title: Block limits overage relaxation
authors: Tao Zhu (Anza)
category: Standard
type: Core
status: Review
created: 2025-06-04
feature:
---

## Summary

This proposal introduces a modification to block CU (Compute Unit) limit
enforcement during block replay, allowing blocks that exceed the CU limit to be
skipped without marking them as invalid and being rejected. This is an enabler
for asynchronous block execution, where voting occurs prior to execution.

## Motivation

Current block execution behavior:

- Transactions to be fully executed before votes are cast,
- Each transaction's CU usage is tracked,
- A block to be marked invalid during replay if the total consumed CU exceeds
  the configured block CU limit.

This behavior ensures strong guarantees around CU consumption but is not
compatible with the upcoming async execution model, where replicas vote before
execution.

To support async execution, the system must gracefully handle the possibility
that a valid voted-on block may exceed limits at replay time, without
invalidating consensus.

It proposes a relaxed approach for block CU enforcement: skip, instead of
reject, a valid voted-on block that exceeds CU limit during replay.

## New Terminology

Skipped Block: a valid, committed block in terms of ledger continuity and votes,
but all transactions in it are treated as having failed, therefore no state
changes at all, nor fee collection, nor nonce advancement.

## Detailed Design

Change the block CU validation logic during replay as follows:

- If a block exceeds the Block CU limit during replay execution:
  - Stop execution of remaining transactions in the block, and rollback all
    previously executed transaction.
  - Do not fail the block.
  - Treat the block as committed but stateless — no accounts are modified, no
    fee collected, no nonce advanced if apply.
  - Maintain consensus liveness and ledger continuity.

## Alternatives Considered

None. Relaxing CU limit enforcement during block replay is a necessary step to
support asynchronous block execution.

## Impact

- Changes only affect validator replay logic, no change required for existing
  users or programs.
- Dapp developers may observe transactions in Skipped Blocks fail silently,
  even Block Producer may have confirmed transaction inclusion.
- Validators may observe additional ledger storage space due to including
  Skipped Blocks (which are dropped currently).

## Security Considerations

- Validators could potentially waste time and storage on Skipped Blocks. This
  is mitigated somewhat due to the block producer will not receive rewards.
  Skipped blocks that available on-chain can also be evidence of slashing when
  necessary.

## Backwards Compatibility

This proposal changes validator replay logic, should be controlled behind a
feature gate, activated only when async execution is enabled.
