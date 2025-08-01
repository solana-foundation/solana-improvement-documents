---
simd: '0329'
title: Account Creation Slot Tracking
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Draft
created: 2025-08-01
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Track the creation slot for each account as part of the account's metadata.

## Motivation

Currently, Solana accounts do not retain information about when they were created. Rent mechanisms like LSR (described [here](https://x.com/aeyakovenko/status/1796569211273445619?lang=en)) require account age to determine the minimum balance for a particular account, which requires knowledge of the creation slot. It helps to include this field now rather than when the rent mechanism itself is introduced because a default value needs to be selected for accounts created before feature activation. The sooner creation slot tracking is activated, the lower this default value can be, which allows for more accurate age calculation when rent is introduced. 

## New Terminology

- **Account Creation Slot**: The slot number in which an account was first created (i.e., when the account's initial allocation occurred).

## Detailed Design

### Account Metadata Extension

Account metadata includes an additional creation slot field encoded as an unsigned little-endian 64-bit integer.

### Implementation Details

1. **New Account Creation**: When a new account is created (via system program allocation or other means), the `creation_slot` field MUST be set to the current slot number.

2. **Snapshot Integration**: The creation slot MUST be included in account data when serializing snapshots and MUST be restored when deserializing snapshots.

3. **Default Value for Existing Accounts**: For accounts that exist before activation of this feature, the `creation_slot` field MUST be set to the slot when activation occurred. This value is the tightest upper bound on the actual creation slot.

4. **Account Reallocation**: If an account is reallocated (expanded or contracted), the `creation_slot` MUST NOT be modified - it represents the original creation, not subsequent modifications.

5. **RPC and Client Exposure**: The creation slot information SHOULD be available through relevant RPC endpoints that return account information, allowing clients to access this metadata.

### Storage Considerations

The additional 8 bytes per account will increase snapshot size. For a network with N accounts, this represents an additional 8*N bytes of storage. This is considered acceptable given the utility of the information.

### Wire Protocol

When transmitted over the network (in snapshots or account data), the creation slot will be serialized as a little-endian 64-bit unsigned integer following existing Solana serialization conventions.

## Alternatives Considered

N/A

## Impact

### Validators
- Increase in snapshot size and memory usage (8 bytes per account)

### Core Contributors
- Foundation for future rent calculation improvements and features downstream of rent (e.g. delinquent account compression)

## Backwards Compatibility

This change is designed to be backwards compatible:

1. **RPC Compatibility**: Existing RPC calls will continue to work. Creation slot information can be added as additional fields in responses without breaking existing clients.

2. **Account Structure**: The creation slot is new metadata and does not modify existing account data or behavior.

3. **Snapshot Compatibility**: New snapshots will include creation slot information and the default creation slot value, but the feature can handle missing creation slot data for accounts migrated from older snapshots by applying the default value.
