---
simd: '0083'
title: Relax Entry Constraints
authors:
  - Andrew Fitzgerald (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2023-11-02
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Remove the constraint that transaction entries cannot contain conflicting
transactions.

## Motivation

The current constraint that transactions within entries must not conflict with
each other is unnecessary.
It is a constraint, that made the current labs-client implementation easier,
but is not required for the protocol to function correctly.
The removal of this constraint simplifies the protocol, and allows more
flexibility in the implementation of both block-produciton and
block-validation.

## Alternatives Considered

1. Do nothing
    - This is the simplest option, as we could leave the protocol as is.
    However, this leaves the protocol more complex than it needs to be.
2. Remove the constraint, but execute conflicting transactions by priority.
    - This is a more complex option than the current proposal.
    - It also gives less flexibility to the block-producer, since transactions
    would be re-ordered.

## New Terminology

None

## Detailed Design

Currently, if a transaction entry within a block contains transactions that
conflict, the entire block is marked invalid.
The proposal is that this constraint is removed entirely, and entries are
allowed to contain transactions that conflict with each other.

If transactions within an entry conflict with each other, they must be
executed sequentially, in the order they appear in the entry.
This decision gives the most freedom in block-production, as it allows for
arbitrary ordering of transactions within an entry by the block-producer.

## Impact

- The protocol is simplified
- Block-production is more flexible

## Security Considerations

This proposal changes the protocol and would break consensus.
This proposal will require a feature-gate, so that all validators can change
behavior at the same time.

## Drawbacks

None beyond the security considerations above.

## Backwards Compatibility

This proposal is backwards compatible with the current protocol, since it only
relaxes constraints, and does not add any new constraints.
All previously valid blocks would still be valid.
However, newly produced blocks may no longer be valid under the old protocol.
