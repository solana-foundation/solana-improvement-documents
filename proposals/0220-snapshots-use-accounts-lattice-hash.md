---
simd: '0220'
title: Snapshots use Accounts Lattice Hash
authors:
  - Brooks Prumo
category: Standard
type: Core
status: Activated
created: 2025-01-08
feature: LTsNAP8h1voEVVToMNBNqoiNQex4aqfUrbFhRH3mSQ2
---

## Summary

Use the **Accounts Lattice Hash** as the **Snapshot Hash**, which enables
removing the merkle-based accounts hash calculation.


## Motivation

This SIMD follows on from SIMD-215, and shares its motivation:

> The main goal is to scale Solana to billions accounts

When snapshots are taken, they contain a **Snapshot Hash**.   This hash is
based on the merkle-based hash of the total account state.  As established in
SIMD-215, this merkle-based hashing of all accounts hinders scaling.  And since
SIMD-215 added the **Accounts Lattice Hash**, which is a hash of the total
account state, the merkle-based hash of all accounts used in the current
**Snapshot Hash** is redundant work.  The **Snapshot Hash** can be updated to
use the **Accounts Lattice Hash** instead.


## New Terminology

None.


## Detailed Design

When constructing the **Snapshot Hash**, the merkle-based hash of all accounts
will be replaced by the **Accounts Lattice Hash**.  And since SIMD-215 removes
the **Epoch Accounts Hash**, the **Snapshot Hash** becomes:

```
snapshot_hash := accounts_lattice_hash.out()
```

Specifically, the **Snapshot Hash** for slot `S` is the 32-byte blake3 of the
**Accounts Lattice Hash** at slot `S`.

Note that SIMD-215 is a prerequisite for this SIMD.


## Alternatives Considered

None.


## Impact

Only validators will be impacted, and their performance will improve since the
merkle-based hashing of all accounts will no longer be performed.


## Security Considerations

None.


## Backwards Compatibility

Snapshots will have a new hash, so consumers of snapshots will need to know
which method to use for hash validation.
