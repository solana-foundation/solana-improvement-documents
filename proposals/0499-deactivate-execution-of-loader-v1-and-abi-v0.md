---
simd: '0499'
title: Deactivate execution of loader-v1 and ABI-v0
authors:
  - Alexander Meißner (Anza)
category: Standard
type: Core
status: Review
created: 2027-03-16
feature: TBD
---

## Summary

Deactivate execution of loader-v1 and ABI-v0.

## Motivation

The trouble with ABIv0 is that it had no alignment padding in its serialization
format and simply ignored any alignment requirements for syscall parameters.
This has since become undefined behavior in Rust. Disabling execution of
loader-v1 would allow us to remove ABIv0 entirely (as it is the only loader
which supports that ABI version) and would reduce the maintenance and auditing
burden in most syscalls significantly.

## New Terminology

None.

## Detailed Design

After the activation of the associated feature key a validator must fail to
execute programs owned by loader-v1, throwing the error message
`TransactionError::InvalidProgramForExecution` during transaction loading.

## Alternatives Considered

Continuing to support this barely used functionality.

## Impact

The only loader-v1 / ABIv0 program still in use today is Memo Program v1,
of which there has been a loader-v2 / ABIv1 replacement around for a long time.
Memo Program v1 usage often occurs in conjunction with Jupiter usage.

One possibility to get users of this program would be to first bump the CU cost
of ABI-v0 significantly in a separate SIMD. Though it is likely unnecessary and
easier to ask them to change their transaction building.

In case the remaining Memo Program v1 traffic can not be migrated to v2, it is
conceivable to adapt the Memo Program v1 (which was written for ABI-v0) to
ABI-v1 and redeploy it on loader-v3 similar to
[SIMD-0418](https://github.com/solana-foundation/solana-improvement-documents/pull/418).

All programs owned by loader-v1 would stop working forever with their locked
funds effectively burned. This might be relevant for sleeper programs which
have not seen any activity in years.

## Security Considerations

The activation of this feature should be straight forward but the later clean
up together with the removal of alignment check skipping would be more complex.

## Drawbacks

Slight inconvenience to the last remaining users of the Memo Program v1 as well
as bricking of any sleeper programs owned by loader-v1.
