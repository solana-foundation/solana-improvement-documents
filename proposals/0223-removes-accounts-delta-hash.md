---
simd: '0223'
title: Removes Accounts Delta Hash
authors:
  - Brooks Prumo
category: Standard
type: Core
status: Activated
created: 2025-01-13
feature: LTdLt9Ycbyoipz5fLysCi1NnDnASsZfmJLJXts5ZxZz
---

## Summary

With the introduction of the **Accounts Lattice Hash** (SIMD-215), the
merkle-based **Accounts Delta Hash** is redundant and can be removed.


## Motivation

This SIMD follows on from SIMD-215, and shares its motivation:

> The main goal is to scale Solana to billions accounts

When a block is done being processed, it is frozen.  The bank hash is
calculated during the freezing process, and includes calculating the **Accounts
Delta Hash**.  The **Accounts Delta Hash** is the merkle-based hash of the
accounts modified in this block.  As established in SIMD-215, merkle-based
hashing of accounts hinders scaling.  And since SIMD-215 added the **Accounts
Lattice Hash**, which is a hash of the total account state, the merkle-based
hash of only the accounts modified per block is redundant.  Therefore the
**Accounts Delta Hash** can be removed from the bank hash entirely.


## New Terminology

None.


## Detailed Design

Remove the **Accounts Delta Hash** from the **Bank Hash**.


## Alternatives Considered

None.


## Impact

Only validators will be impacted, and their performance will improve since the
merkle-based hashing of modified accounts per block will no longer be
performed.


## Security Considerations

None.


## Backwards Compatibility

Incompatible. This changes the bank hash, thus changing consensus.
