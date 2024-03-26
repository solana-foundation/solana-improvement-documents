---
simd: '0133'
title: Syscall Get-Epoch-Stake
authors:
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Draft
created: 2024-03-25
feature: (fill in with feature tracking issues once accepted)
---

## Summary

A syscall to retrieve a vote account's delegated stake for the current epoch.

## Motivation

Currently, on-chain programs have no knowledge of the current epoch's stake and
how much active stake is delegated to a certain vote account.

If this data was available for querying by on-chain programs, it would unblock
many use cases, such as validator governance and secondary consensus mechanisms,
that were previously not possible on Solana.

Additionally, this would enable the Feature Gate program defined in
[SIMD 0089](./0089-programify-feature-gate.md) to tally vote account stake in
support for a pending feature gate.

## Alternatives Considered

An alternative design has been proposed in the past for using an on-chain sysvar
account to store all of the current epoch's stake. It was proposed in
[SIMD 0056](https://github.com/solana-foundation/solana-improvement-documents/pull/56).

Using a sysvar account to store even just the current epoch's stake introduces a
size limitation on the number of entries that can be stored, which the number of
validators on Solana could surpass in the future.

By offering access to this information through only a syscall, we can avoid this
account maximum size constraint. However, the syscall approach also makes the
data more cumbersome to retrieve off-chain than the sysvar approach.

While retrieving this data off-chain may not be straightforward, it remains
entirely feasible. Conversely, on-chain programs currently do not have this
capability at all.

## New Terminology

N/A.

## Detailed Design

The specification for the proposed syscall is as follows:

```c
/**
 * Retrieves the total active stake delegated to a vote account for the current
 * epoch.
 *
 * @param var_addr      VM memory address to copy the retrieved data to.
 * @param vote_address  The vote account whose stake to query.
 * @return              A 64-bit unsigned integer error code:
 *                        - 0 if the operation is successful.
 *                        - Non-zero error code.
 *
 * If the operation is not successful, data will not be written to the
 * provided VM memory address.
 */
uint64_t sol_get_epoch_stake(
  /* r1 */ uint8_t *    var_addr,
  /* r2 */ void const * vote_address,
);
```

`var_addr` must be the starting address of at least 8 bytes of writable VM
memory to store the `u64` response. If not, the syscall will abort the VM with
an access violation.

If the provided vote address corresponds to an account that is not a vote
account or does not exist, the syscall will write `0` for active stake. 

## Impact

This new syscall directly unlocks highly relevant network data for a wide range
of protocols. Developers seeking to access vote account stake will be positively
impacted.

## Security Considerations

This new syscall introduces the same security considerations as the rest of the
syscalls in the existing interface, which manipulate raw pointers to VM memory
and must be implemented with care.

A potential pitfall in the implementation could come when comparing the act of
writing a `0` to the VM memory for active stake versus returning a `0` for
success. The two should not be conflated.

