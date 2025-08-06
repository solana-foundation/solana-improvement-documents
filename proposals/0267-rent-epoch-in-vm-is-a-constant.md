---
simd: '0267'
title: Sets rent_epoch to a constant in the VM
authors:
  - Brooks Prumo
category: Standard
type: Core
status: Activated
created: 2025-03-25
feature: RENtePQcDLrAbxAsP3k8dwVcnNYQ466hi2uKvALjnXx
---

## Summary

Set the value of `rent_epoch`—as serialized into the VM for transaction
processing—to a constant.

## Motivation

The `rent_epoch` field on an account is no longer meaningful.  This is because
all accounts must be rent exempt, and rent fees collection has been disabled
(see SIMD-84).  With this in mind, it would be beneficial to remove
`rent_epoch` from the computation of a single account's hash.

However, before we can remove the `rent_epoch` from the account hash
computation, we must set the value of `rent_epoch`—as serialized into the VM
for transaction processing—to a constant.  If we did not do this, different
validators could have different values for `rent_epoch` passed into the VM,
which could result in different transaction results and thus cluster
divergence.

## New Terminology

None.

## Detailed Design

The value of the `rent_epoch` field serialized into the VM per account shall be
set to the constant used to denote a rent-exempt account.  This value is
`u64::MAX`, aka `0xFFFF_FFFF_FFFF_FFFF`.

## Alternatives Considered

None.

## Impact

The `rent_epoch` field in an account can be deprecated.  This allows reclaiming
and reusing these bytes for other purposes in the future.

In certain rare cases the serialized `rent_epoch` of an account may previously
have been `0` and will now be `u64::MAX`.


## Security Considerations

None.
