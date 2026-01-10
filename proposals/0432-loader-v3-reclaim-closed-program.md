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

The accounts required by the instruction are unchanged:

- Account 0: Programdata account (writable)
- Account 1: Recipient (writable)
- Account 2: Authority (signer)
- Account 3: Program account (writable)

Currently, the authority (index 2) and program (index 3) accounts are only
required for closure of initialized programs. This proposal also requires them
for reclamation of legacy tombstones. This is detailed in the Control Flow
section.

### Base Workflow

For a value of `false`, the program will clear the program account's data,
resize it to zero, and withdraw all lamports. This will render the account no
longer rent-exempt and subject to garbage collection by the runtime at the end
of the transaction. As such, the program address can be reclaimed after the
account has been garbage collected.

The Close instruction MUST fail if `tombstone` is `false` and the program was
deployed in the current slot (this field is stored in the programdata account
layout). This prevents a deploy-close-reclaim loop within the same slot, which
would corrupt the program cache (see Security Considerations). Programs
deployed in the current slot can still be closed with `tombstone=true`.

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

In both workflows, the programdata account will be defunded and set to
`Uninitialized`, causing it to be garbage collected at end of transaction.

### Control Flow

The entire control flow of the Loader V3 `Close` instruction is detailed below
with modifications highlighted.

1. At least 2 accounts must be provided, otherwise throw
   `NotEnoughAccountKeys`.
2. Accounts at index 0 and 1 must not alias, otherwise throw
   `InvalidArgument`.
3. The "close" account (index 0) must deserialize as `UpgradeableLoaderState`,
   otherwise throw `InvalidAccountData`.

Once the close account's state is deserialized, the remaining control flow
depends on the type of account.

#### `ProgramData`

1. At least 4 accounts must be provided, otherwise throw
   `NotEnoughAccountKeys`.
2. The program account (index 3) must be writable, otherwise throw
   `InvalidArgument`.
3. The program account must be owned by Loader v3, otherwise throw
   `IncorrectProgramId`.
4. **[NEW]** If `tombstone` is `false`, the programdata's `slot` field must not
   equal the current slot (from Clock sysvar), otherwise throw
   `InvalidArgument`.
5. The program account must deserialize as `UpgradeableLoaderState`, otherwise
   throw `InvalidAccountData`.
6. The program account must be in `Program` state, otherwise throw
   `InvalidArgument`.
7. The program account's `programdata_address` must match the close account
   (index 0), otherwise throw `InvalidArgument`.
8. If the programdata's `upgrade_authority_address` is `None` (frozen program),
   throw `Immutable`.
9. The authority (index 2) must match the programdata's
   `upgrade_authority_address`, otherwise throw `IncorrectAuthority`.
10. The authority must be a signer, otherwise throw `MissingRequiredSignature`.
11. Transfer all lamports from the close account to the recipient (index 1).
12. Set the close account's state to `Uninitialized` (account will be garbage
    collected at end of transaction due to zero lamports).
13. **[NEW]** Clear the program account's data and resize to zero.
14. **[NEW]** If `tombstone` is `true`: transfer excess lamports (above
    rent-exempt minimum for zero-data account) to recipient, then assign the
    program account to itself.
15. **[NEW]** If `tombstone` is `false`: transfer all lamports from the program
    account to the recipient (account will be garbage collected).

#### `Uninitialized`

First determine if this is a reclaim of a legacy tombstone.

Programs closed before this proposal remain in a legacy tombstone state:

- Program account: Owned by Loader v3, `Program { programdata }` state, funded,
  programdata address points to programdata account.
- Programdata account: `Uninitialized` (all-zeroes).

**[NEW]** This state must be infallibly evaluated. If the above program account
state is confirmed, the control flow for a legacy tombstone reclaim is as
follows:

1. At least 4 accounts must be provided, otherwise throw
   `NotEnoughAccountKeys`.
2. The program account (index 3) must be writable, otherwise throw
   `InvalidArgument`.
3. The program account must be owned by Loader v3, otherwise throw
   `IncorrectProgramId`.
4. The authority (index 2) must equal the program account's pubkey (i.e., the
   program keypair), otherwise throw `IncorrectAuthority`.
5. The authority must be a signer, otherwise throw `MissingRequiredSignature`.
6. Transfer all lamports from the close account to the recipient (index 1).
7. Set the close account's state to `Uninitialized` (account will be garbage
   collected at end of transaction due to zero lamports).
8. Clear the program account's data and resize to zero.
9. If `tombstone` is `true`: transfer excess lamports (above rent-exempt
   minimum for zero-data account) to recipient, then assign the program
   account to itself.
10. If `tombstone` is `false`: transfer all lamports from the program account
    to the recipient.

If this is not a reclaim of a legacy tombstone, the control flow is as follows:

1. At least 1 account must be provided, otherwise throw
   `NotEnoughAccountKeys`.
2. Transfer all lamports from the close account to the recipient (index 1).

#### `Buffer`

1. At least 3 accounts must be provided, otherwise throw
   `NotEnoughAccountKeys`.
2. If the buffer's `authority_address` is `None`, throw `Immutable`.
3. The authority (index 2) must match the buffer's `authority_address`,
   otherwise throw `IncorrectAuthority`.
4. The authority must be a signer, otherwise throw `MissingRequiredSignature`.
5. Transfer all lamports from the close account to the recipient (index 1).
6. Set the close account's state to `Uninitialized` (account will be garbage
   collected at end of transaction due to zero lamports).

#### `Program`

1. Throw `InvalidArgument`. Program accounts cannot be closed directly; the
   programdata account must be closed instead.

### Feature Activation

This change will be a feature-gated behavioral change to the existing Close
instruction. After the feature is activated, the boolean value can be included
to utilize the new functionality, and legacy tombstones can be reclaimed.

## Alternatives Considered

N/A

## Impact

This proposal removes a harmful default behavior that has caused repeated loss
of funds and persistent state bloat, while preserving security guarantees for
users who explicitly wish to permanently disable a program ID.

## Security Considerations

The program cache relies on two invariants:

1. **One redeployment per slot**: The cache keys on program address and
   deployment slot. Multiple deployments to the same address in one slot would
   corrupt the cache.

2. **Loader stability within a transaction**: A program's loader determines its
   ABI and alignment requirements. Changing loaders mid-transaction would cause
   CPI mismatches.

This proposal preserves both invariants:

- **Program account**: When closing without tombstoning, the account is drained
  of lamports rather than reassigned to System. The account remains owned by
  Loader v3 until garbage-collected at transaction end, preventing same-TX
  redeployment. Additionally, closing without tombstoning is rejected if the
  program was deployed in the current slot, preventing multiple-TX loops.

- **Programdata account**: Fully deallocated and reassigned to System. This is
  safe because programdata is not used for cache indexing or invocation.

- **Tombstone**: When tombstoning, the program account is assigned to itself,
  permanently locking the address. Self-owned accounts cannot be modified.

## Backwards Compatibility

This change modifies the semantics of an existing Loader v3 instruction and
therefore requires a feature gate for consensus safety.

From a tooling perspective, the change is backwards compatible, though tooling
updates are required to access the new explicit tombstoning behavior.
