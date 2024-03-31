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
how much active stake is delegated to any vote account.

If this data were available for querying by on-chain programs, it would unblock
many use cases, such as validator governance and secondary consensus mechanisms,
that were previously not possible on Solana.

Additionally, this would enable the Feature Gate program defined in
[SIMD 0089](./0089-programify-feature-gate.md) to tally vote account stake in
support for a pending feature gate.

## Alternatives Considered

[SIMD 0056](https://github.com/solana-foundation/solana-improvement-documents/pull/56)
proposes using an on-chain sysvar account to store all of the current epoch's
stake.

Because account data is finite, using a sysvar account to store even just the 
current epoch's stake limits the number of entries that can be stored. The
amount of validators in one Solana cluster could surpass this number in the
future.

Exposing epoch-stake information through a syscall avoids this account maximum
size constraint. While the syscall approach does not offer the easy off-chain
retrieval of a sysvar, there are existing ways to get epoch-stake data off
chain. The priority of a new design should be making the data available to
on-chain programs.

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
 * @param vote_address  A pointer to 32 bytes representing the vote address.
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

`var_addr` must be the starting address of 8 bytes of writable VM memory to
store the `u64` response. If not, the syscall will abort the VM with an access
violation.

If the provided vote address corresponds to an account that is not a vote
account or does not exist, the syscall will write `0` for active stake. 

### Compute Unit Usage

The syscall will always attempt to consume the same amount of CUs regardless of
control flow.

```
(32 / cpi_per_u) + (8 / cpi_per_u)
```

- `syscall_base`: Base cost of a sysvall.
- `cpi_per_u`: Number of account data bytes per CU charged during CPI.

## Impact

Dapp developers will be able to query vote account stake for the current epoch
from within on-chain programs.

## Security Considerations

This new syscall introduces the same security considerations as the rest of the
syscalls in the existing interface, which manipulate raw pointers to VM memory
and must be implemented with care.

A potential pitfall in the implementation could come when comparing the act of
writing a `0` to the VM memory for active stake versus returning a `0` for
success. The two should not be conflated.

