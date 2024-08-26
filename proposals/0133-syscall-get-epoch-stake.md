---
simd: '0133'
title: Syscall Get-Epoch-Stake
authors:
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Implemented
created: 2024-03-25
feature: [7mScTYkJXsbdrcwTQRs7oeCSXoJm4WjzBsRyf8bCU3Np](https://github.com/anza-xyz/agave/issues/884)
development:
  - Anza - [Implemented](https://github.com/solana-foundation/solana-improvement-documents/pull/133)
  - Firedancer - Implemented
---

## Summary

A syscall to retrieve a vote account's delegated stake or the total cluster
stake for the current epoch.

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
 * Retrieves the total active stake delegated to a vote account, or for the
 * entire cluster, for the current epoch.
 *
 * If a valid pointer for `vote_addr` is provided, returns the total active
 * stake delegated to the vote account at the 32-byte address found at
 * `var_addr`.
 *
 * If a null pointer is provided, returns the total active stake for the
 * cluster.
 *
 * @param vote_addr     A pointer to 32 bytes representing the vote address.
 *                      Can be a null pointer.
 * @return              A 64-bit unsigned integer representing the total
 *                      active stake delegated to the vote account at the
 *                      provided address.
 */
uint64_t sol_get_epoch_stake(/* r1 */ void const * vote_addr);
```

### Control Flow

If `var_addr` is _not_ a null pointer:

- The syscall aborts the virtual machine if:
    - Not all bytes in VM memory range `[vote_addr, vote_addr + 32)` are
      readable.
    - Compute budget is exceeded.
- Otherwise, the syscall returns a `u64` integer representing the total active
  stake delegated to the vote account at the provided address.
  If the provided vote address corresponds to an account that is not a vote
  account or does not exist, the syscall will return `0` for active stake.

If `var_addr` is a null pointer:

- The syscall aborts the virtual machine if:
    - Compute budget is exceeded.
- Otherwise, the syscall returns a `u64` integer representing the total active
  stake on the cluster for the current epoch.

### Compute Unit Usage

The syscall will always consume a fixed amount of CUs regardless of control
flow. This fixed amount can be one of two values, depending on whether a null
pointer was provided for `var_addr`.

If `var_addr` is _not_ a null pointer:

```
syscall_base + floor(PUBKEY_BYTES/cpi_bytes_per_unit) + mem_op_base
```

If `var_addr` is a null pointer:

```
syscall_base
```

- `PUBKEY_BYTES`: 32 bytes for an Ed25519 public key.
- `syscall_base`: Base cost of a syscall.
- `cpi_bytes_per_units`: Number of account data bytes per CU charged during CPI.
- `mem_op_base`: Base cost of a memory operation syscall.

## Impact

Dapp developers will be able to query vote account and cluster stake for the
current epoch from within on-chain programs.

## Security Considerations

This new syscall introduces the same security considerations as the rest of the
syscalls in the existing interface, which manipulate raw pointers to VM memory
and must be implemented with care.

