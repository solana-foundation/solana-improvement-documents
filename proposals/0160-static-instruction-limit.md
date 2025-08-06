---
simd: '0160'
title: Static Instruction Limit
authors:
  - Andrew Fitzgerald (anza)
category: Standard
type: Core
status: Accepted
created: 2024-07-24
feature:
supersedes:
superseded-by:
extends:
---

## Summary

Transactions will fail execution in SVM if the transactions use more than 64
instructions, including CPI calls.
This proposal is to make transactions that use more than 64 top-level
instructions fail sanitization checks and be rejected by the network.

## Motivation

The current limit of 64 instructions is a runtime failure, and makes sense
in the context of CPIs.
It is bad user experience to allow creation and submission of transactions that
have no chance of being executed successfully.
Several other checks worst-case performance scales with the number of
top-level instructions in a transaction, so limiting this number can help the
worst-case performance.

## Alternatives Considered

- Do nothing.
- Allow transactions with more than 64 top-level instructions to be included in
  blocks, but skip execution only taking fees.
  - Checking the number of top-level instructions in a transaction is a
    relatively cheap operation and can be done very early in the processing of
    a transaction, similar to the current check that the number of required
    signatures matches the number of signatures provided.
  - Additionally, if we still allow more than 64 top-level instructions in a
    transaction we would still need to parse all the instructions to determine
    the fee, so there is a performance benefit in strictly limiting the number
    of top-level instructions.

## New Terminology

None.

## Detailed Design

Any transaction that has more than 64 top-level instructions cannot be included
in a block.
If a block contains a transaction with more than 64 top-level instructions, the
block must be marked as invalid.

## Impact

- Users are prevented from creating transactions that cannot be executed successfully.
- Smaller upper bound on number of instructions can help performance of validator.

## Security Considerations

- Requires a feature-gate to enable the new limit.

## Drawbacks *(Optional)*

- Similar to the runtime check on number of instructions including CPI, logic
  is duplicated.
- Can no longer collect fees from transactions that are rejected due to new
  limit.

## Backwards Compatibility *(Optional)*

- Some transactions that are currently valid will be rejected.
