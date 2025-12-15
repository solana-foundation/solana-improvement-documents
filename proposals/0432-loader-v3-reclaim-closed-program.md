---
simd: '0432'
title: 'Loader V3: Reclaim Closed Program'
authors:
    - Joe Caulfield (Anza)
    - Dean Little (Blueshift)
category: Standard
type: Core
status: Review
created: 2025-12-14
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD proposes changing the default behavior when closing upgradeable
programs so that program accounts are fully reclaimed and their addresses
become reusable. Tombstoning program accounts would remain supported, but only
when explicitly requested.

## Motivation

Today, closing an upgradeable program permanently tombstones its program
account, preventing reuse of the program ID. This behavior has led to several
issues:

- Loss of funds: Users frequently confuse close program with close buffer in the
  Solana CLI, unintentionally irreversibly disabling programs.
- Permanent account bloat: Tombstoned program accounts cannot be reclaimed and
  accumulate indefinitely in the accounts database.
- RPC performance degradation: getProgramAccounts against the loader v3 program
  must return all program accounts, including closed ones, increasing response
  size and latency.

These drawbacks outweigh the benefits of mandatory tombstoning. A more flexible
model allows safe address reuse by default while preserving explicit
tombstoning for users who require it.

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

The `Close` instruction will be updated to include an optional boolean input. If
not provided, the default will be `false`.

```
Close { tombstone: bool }
```

```
| 4-byte discriminator | 1-byte boolean |
```

For a value of `false`, the program will clear the program account's data,
resize it to zero, and withdraw all lamports. This will render the account no
longer rent-exempt and subject to garbage collection by the runtime at the end
of the transaction. As such, the program address can be reclaimed after the
account has been garbage collected.

For a value of `true`, the program will clear the program account's data, resize
it to zero, but retain the rent-exempt minimum lamports for the base account
metadata. The program account will then be assigned to itself, creating a
permanent tombstone for the program.

```
                         Close { tombstone }
                                |
                    +-----------+-----------+
                    |                       |
             tombstone=false          tombstone=true
                    |                       |
           Clear data & resize      Clear data & resize
           Withdraw all lamports    Retain rent-exempt min
                    |                       |
           Account → GC'd           Owner → self (tombstone)
           Address reclaimable      Address permanently locked
```

In both workflows, the programdata account (or any adjacent accounts under
Loader v3) will be completely deallocated, defunded, and reassigned to System.

This change will be a feature-gated behavioral change to the existing Close
instruction. After the feature is activated, the boolean value can be included
to utilize the new functionality.

## Alternatives Considered

N/A

## Impact

This proposal removes a harmful default behavior that has caused repeated loss
of funds and persistent state bloat, while preserving security guarantees for
users who explicitly wish to permanently disable a program ID.

## Security Considerations

When closing a program, the program account must not be reassigned to the
system program. Doing so would allow redeployment within the same slot,
potentially corrupting the program cache. When tombstoning, assigning the
account to itself ensures the address is permanently unusable, as self-owned
accounts cannot be modified by any program.

## Backwards Compatibility

This change modifies the semantics of an existing Loader v3 instruction and
therefore requires a feature gate for consensus safety.

From a tooling perspective, the change is backwards compatible, though tooling
updates are required to access the new explicit tombstoning behavior.
