---
simd: '0339'
title: Increase CPI Account Info Limit
authors:
  - Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2025-08-15
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Increase the maximum account info length for cross-program invoked (CPI)
instructions from 64 to 255 so that onchain programs can invoke CPI's without
first needing to deduplicate account info's.

## Motivation

CPI's are restricted to a limit of 64 account info's passed to the syscall.
This limit is burdensome for onchain programs which themselves were invoked with
more than 64 accounts because it means they cannot simply pass through the same
list of account info's that they were invoked with to another CPI syscall. They
are faced with the burden of first deduplicating the account info's and
constructing a new list before making the syscall.

This problem arises when onchain programs wrap another program (such as Jupiter)
that composes many other programs and are invoked with over 64 accounts
(including duplicates).

## New Terminology

NA

## Detailed Design

Increase the max account info length imposed on CPI syscalls from 64 to 255.
Note that this max is inclusive, meaning that account info's with a length of
255 is valid.

## Alternatives Considered

What alternative designs were considered and what pros/cons does this feature
have relative to them?

## Impact

Since the list of account info's passed to a CPI can now be ~4 times longer,
there will be more overhead in the SVM to map each instruction account address
to one of the account info.

## Security Considerations

NA

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

The max account info length increase for CPI's will be feature gated. Since the
limit is being increased, existing behavior will not be restricted.
