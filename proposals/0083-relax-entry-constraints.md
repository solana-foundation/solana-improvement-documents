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

Remove the constraint that transaction entries must not conflict with each
other.

## Motivation

The current constraint that transaction entries must not conflict with each
other is unnecessary.
It is a constraint, that made the current labs-client implementation easier,
but is not required for the protocol to function correctly.
The removal of this constraint simplifies the protocol, and allows more
flexibility in the implementation of both block-produciton and
block-validation.

## Alternatives Considered

1. Do nothing
    - This is the simplest option, as we could leave the protocol as is.
    However, this leaves the protocol more complex than it needs to be.

## New Terminology

None

## Detailed Design

Currently, if a transaction entry within a block contains transactions that
conflict, the entire block is marked invalid.
The proposal is that this constraint is removed entirely, and entries are
allowed to contain transactions that conflict with each other.

## Impact

- The protocol is simplified
- Block-production is more flexible

## Security Considerations

None

## Drawbacks

None

## Backwards Compatibility

This proposal is backwards compatible with the current protocol, since it only
relaxes constraints, and does not add any new constraints.
All previously valid blocks would still be valid.
