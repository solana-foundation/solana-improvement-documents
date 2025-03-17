---
simd: '0224'
title: Remove Rent Epoch from Accounts Hash
authors:
  - Tom Pointon
category: Standard
type: Core
status: Review
created: 2025-03-17
feature: TBD
---

## Summary

Now all accounts are rent-exempt, the **Rent Epoch** field in the account
metadata is redundant, and can be removed from the **Accounts Hash**.

## Motivation

The **Rent Epoch** field is redundant, and should be removed from the
**Accounts Hash**.
This will allow greater flexbility for how we use these bytes in the account
stores in the future, without having to worry about backwards compatibility.

Removing the **Rent Epoch** field from the **Accounts Hash** also
allows the complete removal of all rent-related code, simplifying client
implementations.

## New Terminology

None.


## Detailed Design

Remove the **Rent Epoch** field from the **Accounts Hash**.


## Alternatives Considered

None.


## Impact

Only validators will be impacted, but the impact should be negligible as this
is just a change to the Accounts Hash.

## Security Considerations

None.


## Backwards Compatibility

Incompatible. This changes the bank hash, thus changing consensus.
