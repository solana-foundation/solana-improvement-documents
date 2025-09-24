---
simd: '0341'
title: v0 Account Compression
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-08-21
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Protocol-level account compression system including a new system
program to handle compression and decompression requests. The goal is to
significantly reduce the active account state and snapshot size by removing
qualifying accounts in such a way that they can be subsequently recovered. 

## Motivation

Solana's current account model requires all account data to be stored in
full on-chain, replicated on all validators indefinitely, leading to
significant storage costs and blockchain state bloat. Rent/storage cost is
already a significant complaint among app developers, and without reducing
the state size or growth, rent cannot be safely lowered and optimizations
like fully in-memory account state are infeasible in the medium to long
term. To solve this in an enduring way, we need:

1. an economic mechanism to limit state growth
2. a predicate to determine which accounts can be compressed
3. a compression scheme that removes accounts from the global state while
   allowing for safe and simple recovery.

This proposal focuses on (3), leaving (1) and (2) for other SIMDs.

## New Terminology

- **Compression**: replacing arbitrary account data with a fixed size
  commitment. Not to be confused with traditional compression; the data
  cannot be directly recovered from compressed state.
- **Compression Condition**: a predicate determining whether or not an
  existing account can be compressed. A specific compression condition isn't
  provided in this SIMD -- it is assumed to always be false, meaning no
  account is eligible for compression.
- `data_hash = lthash.out(Account)` where Account includes the pubkey, 
  lamports, data, owner, executable, and rent_epoch fields. See 
  [SIMD-0215](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0215-accounts-lattice-hash.md)
  for the definition of the lattice hash functions.
- **Decompression**: the process of restoring a compressed account to its
  original state by providing the original account data and verifying it
  matches the stored data_hash

## Detailed Design

### Syscalls for Compression Operations

The following new syscalls will be introduced to support account compression
operations. The compression system program will support two instructions that
just wrap these syscalls. For the time being, these syscalls can only be used
from the compression system program but that constraint may be relaxed in the
future.

#### `sol_compress_account(account_pubkey: &[u8; 32])`

**Purpose**: Compresses an existing account by replacing its data with a
cryptographic commitment.

**Parameters**:

- `account_pubkey`: pubkey of the account to be compressed

**Behavior**:

- MUST verify that the caller is the hardcoded compression system program
- MUST verify the provided account satisfies the compression condition
- MUST mark the account as compressed if verification succeeds
- if the transaction containing the compression request succeeds, all
  subsequent attempts to access the account MUST fail unless the account has
  been decompressed.

While marking the account as compressed must be done synchronously, the
actual compression (ie the full replacement of the account with its
compressed form in the account database) can be done asynchronously for
performance:

- compute the 32-byte `data_hash`
- replace the account in account databse with a compressed account entry
- (optional) emit a compression event for off-chain data archival

#### `sol_decompress_account(..)`

**Purpose**: Recovers a compressed account by restoring its original data.

**Parameters**:

- `account_pubkey`: 32-byte public key of the account to decompress
- `lamports`: The lamport balance of the original account
- `data`: Pointer to the original account data bytes
- `data_len`: Length of the account data in bytes
- `owner`: 32-byte public key of the account owner
- `executable`: Whether the account is executable
- `rent_epoch`: The rent epoch of the original account

**Behavior**:

- MUST verify the caller is the hardcoded compression system program
- MUST verify the account is currently in compressed state
- MUST compute `lthash.out(Account)` from the provided parameters and verify
  it matches the stored compressed account's data_hash
- if verification succeeds, the original account MUST be restored to the active set
    - this is treated exactly like a new account allocation so rent
      requirements, load limits, etc all apply
- if verification succeeds, the compressed account entry must be replaced
  with the full account data

### Database and Snapshot Extensions

#### Compressed Accounts Storage

Compressed accounts are stored directly in the account database like regular
accounts, but with a special compressed account structure:

```rust
pub struct CompressedAccount {
    pub pubkey: Pubkey,
    pub data_hash: [u8; 32],
}
```

#### Bank Hash Integration

The bank hash calculation is updated to handle compressed accounts:

- **Existing behavior**: All accounts continue to contribute to the bank hash
  via the accounts lattice hash
- **New behavior**: Compressed accounts contribute to the lattice hash using
  their compressed representation instead of full account data
- **Hash calculation**: No changes to the overall bank hash structure, but
  the lattice hash computation includes compressed accounts:

```
lthash(account: CompressedAccount) :=
    lthash.init()
    lthash.append( account.pubkey )
    lthash.append( account.data_hash )
    return lthash.fini()
```

### Account Creation Validation

When creating new accounts, the runtime MUST verify the target pubkey does not
already exist as a compressed account, just like with uncompressed accounts.

#### Execution Error

If an attempt is made to create an account at a pubkey that already exists as
a compressed account, the transaction MUST fail with the `AccountAlreadyInUse`
system error.

This maintains consistency with existing Solana behavior where any attempt to
create an account at an occupied address fails with the same error, regardless
of whether the existing account is active or compressed.

It may be worthwhile to introduce a new system error specific to collisions
on compressed public keys only if that's more clearly actionable for users
and developers.

### Off-chain Data Storage

Since compressed accounts only store the data hash on-chain, the original
account data must be stored off-chain for recovery purposes. This system
provides multiple mechanisms for data availability:

#### RPC Provider Storage

RPC providers can maintain archives of compressed account data to support
client applications. When an account is compressed, the original data is made
available through RPC endpoints for future recovery operations.

#### Account Subscription for Compression Events

The existing `accountSubscribe` RPC endpoint will be extended to notify
subscribers when accounts are compressed. This provides real-time access to
compression events through the established subscription mechanism.

When an account is compressed, subscribers will receive:

```typescript
interface AccountNotification {
  // ... existing fields
  result: {
    context: {
      slot: number;
    };
    value: CompressedAccountInfo | ActiveAccountInfo;
    originalAccount?: {  // Only included during compression events
      pubkey: string;
      lamports: number;
      data: Uint8Array;
      owner: string;
      executable: boolean;
      rentEpoch: number;
    };
  };
}
```

**Critical Implementation Detail**: Validators MUST NOT delete the full account
data until all `accountSubscribe` subscribers have been notified of the
compression event. This ensures that off-chain services have the opportunity
to archive the original account data before it is permanently removed.

This approach enables:

- **Archive services**: Third-party services can maintain comprehensive
  compressed data archives using existing subscription infrastructure
- **Application-specific storage**: DApps can store their own compressed
  account data through established patterns
- **Redundancy**: Multiple parties can maintain copies for data availability

### RPC Handling for Compressed Accounts

Existing RPC endpoints must be updated to handle compressed accounts properly.
When a client requests account information for a compressed account, the
response should clearly indicate the compression status and provide the
available data.

#### Updated `getAccountInfo` Response

The existing `getAccountInfo` endpoint will return modified responses for
compressed accounts:

```typescript
interface CompressedAccountInfo {
  compressed: true;
  pubkey: string;
  dataHash: string;  // 32-byte hex-encoded lattice hash
}

interface ActiveAccountInfo {
  compressed: false;
  lamports: number;
  data: [string, string];  // existing format [data, encoding]
  owner: string;
  executable: boolean;
  rentEpoch: number;
}

type AccountInfo = CompressedAccountInfo | ActiveAccountInfo;
```

#### (optional) New `getCompressedAccountData` Endpoint

A new RPC endpoint specifically for retrieving compressed account data.
This is optional as it requires retaining data that isn't relevant to
core validator operations.

**Endpoint**: `getCompressedAccountData`
**Method**: POST
**Parameters**:

- `pubkey`: string - Account public key
- `commitment?`: Commitment level
- `dataHash?`: string - Optional data hash for verification

**Response**:

```typescript
interface CompressedAccountDataResponse {
  pubkey: string;
  dataHash: string;
  originalAccount: {
    pubkey: string;
    lamports: number;
    data: Uint8Array;
    owner: string;
    executable: boolean;
    rentEpoch: number;
  } | null;  // null if data not available
}
```

### Performance Considerations

- **Recovery operations**: will require a disk-read so CU cost should be set
  accordingly.
- **Off-chain storage**: Applications and RPC providers need sufficient
  storage capacity for compressed account archives.
- **RPC performance**: Compressed account queries may require additional
  archive lookups, potentially increasing response times.

## Alternatives Considered

### Fixed-size vector commitments

All compressed data is moved off-chain and replaced with a fixed size vector
commitment. Membership/Non-membership proofs are used for account creation,
compression, and decompression.

**Pros**: Minimal on-chain storage

**Cons**: Complex proof generation, proof availability concerns, complexity

The next iteration of account compression will likely look similar to this
but the complexity isn't currently necessary.

### Reduce and fix hot-set size with a chili peppers-like approach

**Pros**: keeps all data on-chain, no need for users to manually recover old
accounts

**Cons**: doesn't reduce total state size so snapshots remain large and
rent remains high

Chili peppers has other applications but may not be necessary if account
compression can reduce the global state size sufficiently to store the
entire account state in memory.

### Conclusion

The proposed hash-based approach provides the optimal balance of storage
savings, performance predictability, and implementation complexity.

## Impact

### DApp and Wallet Developers

- include checks for account compression status in program interaction
  workflow
- add instructions to transactions for account recovery when appropriate
- additional regular programs can be deployed to wrap CPI calls to the
  compression system program to improve UX. For example, a decompression
  request program can allow users to submit accounts they would like to be
  decompressed along with a tip to incentivize others to fulfill the request.

### Validators

- **Memory/Storage savings**: If enough accounts are compressed, a
  significant reduction in disk/memory usage is expected
- **Network impact**: Reduced snapshot sizes improve sync times

### Core Contributors

- **Implementation scope**: Changes required in runtime, banking, and
  snapshot systems
- **Testing requirements**: Comprehensive testing of compression/recovery cycles
- **Monitoring needs**: New metrics for compression performance, compressed
  state size, rate, etc

## Security Considerations

### Data Integrity

- **Hash verification**: All recovery operations verify data integrity via
  lattice hash comparison
- **Atomic operations**: Compression/recovery operations are atomic to
  ensure consistent state across the cluster

### Attack Vectors

- **Hash collision attacks**: collisions on the `data_hash` would allow for
  introducing arbitrary data into the account state. The lattice hash function
  provides sufficient collision resistance.

## Backwards Compatibility

This feature introduces breaking changes:

### Bank Hash Changes

- **Impact**: Bank hash calculation includes compressed accounts, affecting
  consensus
- **Mitigation**: Feature gate activation ensures all validators adopt
  simultaneously

### Snapshot Format

- **Impact**: New snapshot format including compressed accounts
- **Mitigation**: Version-aware snapshot loading with backward compatibility
  for old snapshots

### Account Creation Behavior  

- **Impact**: Account creation fails if pubkey already exists as a compressed
  account
- **Mitigation**: if the account corresponding to the pubkey was previously
  compressed it must be recovered rather than recreated. RPCs and good errors
  can provide the relevant info.
