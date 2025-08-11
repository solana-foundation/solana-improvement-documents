---
simd: '0329'
title: Account Access and Creation Epoch Tracking
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-08-01
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

- Track the creation epoch for each account as part of the account's metadata.
- For accounts created before this feature is activated, also track which have
  been accessed since feature activation in `fresh_pre_activation_accounts`.
  These accounts will have their creation epoch default to `activation_epoch`.

In the future, accounts with `creation_epoch` set to the default
`activation_epoch` and not present in `fresh_pre_activation_accounts` will be
eligible for first-pass compression.

## Motivation

Currently, Solana accounts do not retain information about when they were
created. Rent mechanisms like LSR (described
[here](https://x.com/aeyakovenko/status/1796569211273445619?lang=en)) require
account age to determine the minimum balance for a particular account, which
requires knowledge of the creation slot or epoch. It helps to include this field
now rather than when the rent mechanism itself is introduced because a default
value needs to be selected for accounts created prior to feature activation.
The sooner creation epoch tracking is activated, the lower this default value
can be, which allows for more accurate age calculation when rent is introduced.

Additionally, account compression (also described in the article above) requires
some condition accounts must satisfy in order to be eligible for compression.
Eventually LSR may be used for this purpose, where rent delinquency is the
compression condition. In lieu of that, a simple first-pass compression
condition can be used: if an account's `creation_epoch` is set to the default
`activation_epoch` and not present in `fresh_pre_activation_accounts`, it is
eligible for compression.

It's estimated that ~80% of solana state hasn't been accessed for several months
(todo: include specific numbers when relevant data has been collected). Because
of this, first-pass compression can significantly reduce the active state size
without requiring any complex rent mechanism to provide input to the compression
condition.

## New Terminology

- **`activation_epoch`**: the epoch number in which the `creation_epoch`
  tracking feature was activated.
- **Account `creation_epoch`**: the epoch number in which an account was first
  created (i.e. when the account's initial allocation occurred). Defaults to
  `activation_epoch` for accounts created before feature activation.
- **`fresh_pre_activation_accounts`**: a set of all pre-activation accounts
  (e.g. those with `creation_epoch` set to the default `activation_epoch`) that
  have been read or written to in a transaction since feature activation.
- **First-Pass Compression**: in the future, pre-activation accounts not present
  in `fresh_pre_activation_accounts` will be eligible for compression. This is
  a one-time compression sweep, so accounts exempt from first-pass compression
  will remain so indefinitely. They will not necessarily be exempt for future
  delinquency-based compression.

## Detailed Design

### Account Metadata Extension

Account metadata includes an additional `creation_epoch` field encoded as an
unsigned little-endian 3-byte integer.

#### Implementation Details

1. **New Account Creation**: When a new account is created (via system
   program allocation or other means), the `creation_epoch` field MUST be set
   to the current epoch number.

2. **Snapshot Integration**: `creation_epoch` MUST be included in account
   data when serializing snapshots and MUST be restored when deserializing
   snapshots.

3. **Default Value for Existing Accounts**: For accounts that exist before
   activation of this feature, the `creation_epoch` field MUST be set to
   `activation_epoch`. This value is the tightest upper bound on the actual
   creation epoch.

4. **Account Reallocation**: If an account is reallocated (expanded or
   contracted), the `creation_epoch` MUST NOT be modified - it represents the
   original creation, not subsequent modifications.

5. **RPC and Client Exposure**: The creation epoch information SHOULD be
   available through relevant RPC endpoints that return account information,
   allowing clients to access this metadata.

#### Storage Considerations

The additional 3 bytes per account may increase snapshot size if insufficient
unused account metadata bytes are available. Currently, there are enough unused
bytes (due to padding and deprecated fields) so this specific change shouldn't
increase state size.

### Pre-activation Account Tracking

After feature activation, when a transaction reads or writes to an account with
`creation_epoch` set to `activation_epoch`, a unique identifier for that account
must be added to `fresh_pre_activation_accounts`.

#### Implementation Details

1. **Snapshot Integration**: `fresh_pre_activation_accounts` MUST be stored in
   the snapshot as it will be used later for compression eligibility checks.
   It's stored as a simple vector with no account identifier appearing more than
   once.

2. **Deterministic Replication**: for the same reasons, all validators MUST
   agree on the exact contents of `fresh_pre_activation_accounts`.

3. **Truncation**: it's unnecessary to store the full 32-byte pubkey for
   accounts added to `fresh_pre_activation_accounts`. Instead a truncated
   10-byte identifier is used to save space. This can be the first 10 bytes of
   the account pubkey. Collisions are very unlikely for 1B accounts, but if one
   occured the effect would be small: an account that would've been otherwise
   eligible for first-pass compression becomes exempt.

4. **RPC and Client Exposure**: May or may not be exposed through RPC.

#### Storage Considerations

Assuming the entire state of the solana blockchain (~1B accounts) is accessed
post-activation (representing the worst-case) `fresh_pre_activation_accounts`
will have a size of ~10GB.

## Alternatives Considered

N/A

## Impact

### Validators

- Marginal increase in memory usage and snapshot size.

### Core Contributors

- Foundation for future rent calculation improvements and features downstream
  of rent (e.g. delinquent account compression).
- Enables simple first-pass account compression for stale accounts.

## Security Considerations

N/A

## Backwards Compatibility

This change is designed to be backwards compatible:

1. **RPC Compatibility**: Existing RPC calls will continue to work. Creation
   epoch information can be added as additional fields in responses without
   breaking existing clients.

2. **Account Structure**: The creation epoch is new metadata and does not
   modify existing account data or behavior. Unused bytes and bytes from
   deprecated fields will be used to store this value.

3. **Snapshot Compatibility**:
   1. New snapshots will include creation epoch information as part of the
   account metadata (e.g. in `AccountSharedData`). If the snapshot was created
   before feature activation, `creation_epoch` defaults to `activation_epoch`. 
   2. A new, independent data structure will be added to the snapshot
   representing `fresh_pre_activation_accounts`. Pre-activation snapshots will
   initialize this to the empty set.
