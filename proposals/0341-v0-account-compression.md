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
- `account_hash = sha_256(account_pubkey)[0..10]`
- `data_hash = sha_256(bincode_serialize(AccountSharedData))`
- **Compressed Accounts Map (CAM)**: for each compressed account, maps the
  `account_hash` to the `data_hash`.

## Detailed Design

### Syscalls for Compression Operations

The following new syscalls will be introduced to support account compression
operations. The compression system program will support two instructions that
just wrap these syscalls.

#### `sol_compress_account(account_pubkey: &[u8; 32])`

**Purpose**: Compresses an existing account by replacing its data with a
cryptographic commitment.

**Parameters**:

- `account_pubkey`: pubkey of the account to be compressed
- `combined_hash`: 32-byte hash of concatenated account pubkey and current
  account data

**Behavior**:

- MUST verify that the caller is the hardcoded compression system program
- MUST verify the provided account satisfies the compression condition
- MUST mark the account as compressed if verification succeeds
    - this is left as an implementation detail: the `CAM` is the in-protocol
      authority on the compressed set but client implementations can, for
      example, maintain a small in-memory set of compressed accounts that
      haven't yet been added to the `CAM`. When a snapshot is created at a
      particular slot, the `CAM` must be up to date as of that slot.
- if the transaction containing the compression request succeeds, all
  subsequent attempts to access the account MUST fail unless the account has
  been decompressed.

While marking the account as compressed must be done synchronously, the
actual compression (ie the full replacement of the account with its
compressed form stored in the `CAM`) can be done asynchronously for
performance:

- compute the 10-byte `account_hash`
- compute the 32-byte `data_hash`
- assign `data_hash` to `CAM[account_hash]`

#### `sol_decompress_account(original_data: *const u8, data_len: u64)`

**Purpose**: Recovers a compressed account by restoring its original data.

**Parameters**:

- `original_data`: Pointer to the bincode serialization of the original
  `AccountSharedData` object, which includes the pubkey
- `data_len`: Length of the original data in bytes

**Behavior**:

- MUST verify the caller is the hardcoded compression system program
- MUST verify the account is currently in compressed state by computing the
  `account_hash` from the provided `original_data` and checking the `CAM`
- MUST verify the `data_hash` computed from the provided `original_data`
  matches `CAM[account_hash]`
- if verification succeeds, the original account MUST be restored to the active set
    - this is treated exactly like a new account allocation so rent
      requirements, load limits, etc all apply
- if verification succeeds, the `pubkey_hash` must be removed from the `CAM`

### Database and Snapshot Extensions

#### Compressed Accounts Storage

The data structure used for the `CAM` in the runtime is left as an
implementation detail, but it MUST be stored in the snapshots as a
`Vec<CompressedAccountEntry>`:

```rust
pub struct CompressedAccountEntry {
    pub account_hash: [u8; 10],
    pub data_hash: [u8; 32],
}
```

The `CAM` MUST be:

- **Persisted**: Included in snapshots and incremental snapshots
- **Versioned**: Track compression/decompression operations across slots
- **Fork Aware**: Track compressed account data across different forks. 

#### Bank Hash Integration

The bank hash calculation MUST include the `CAM`.

- **Existing behavior**: All non-compressed accounts continue to contribute
  to the bank hash via the accounts lattice hash or merkle root
- **New behavior**: A new lattice hash, `compressed_lattice_hash_bytes`,
  committing to the state of the `CAM` is maintained and also rolled into
  the bank hash.
- **Hash calculation (simplified)**: 
    - current: `bank_hash = H(H(parent_hash, signature_count, last_blockhash),
      accounts_lattice_hash_bytes)`
    - new: `bank_hash = H(H(parent_hash, signature_count, last_blockhash),
      accounts_lattice_hash_bytes, compressed_lattice_hash_bytes)`
    - the important distinction here is that the compressed lattice hash
      immediately follows the uncompressed lattice hash

```
compressed_lattice_hash_bytes = sum(lthash(entry) for entry in CAM)

lthash(entry: CompressedAccountEntry) :=
    lthash.init()
    lthash.append( entry.pubkey_hash )
    lthash.append( entry.data_hash )
    return lthash.fini()
```

#### Snapshot Format Changes

Snapshots will include a new section `compressed_accounts` containing the
full compressed account state as a `Vec<CompressedAccountEntry>`

### Account Creation Validation

When creating new accounts, the runtime MUST verify the target pubkey is not
in the `CAM`. Because the `CAM` will likely be stored on disk to limit memory
usage, an efficient in-memory data structure should be used to verify
non-membership. A counting bloom filter, cuckoo filter, or similar data
structure that supports deletions can be used but must be machine-diversified
to prevent a cluster-wide slow disk read.

### Performance Considerations

- **Recovery operations**: will require a disk-read so CU cost should be set
  accordingly.

## Alternatives Considered

### fully off-chain storage of compressed account data with only fixed-size
vector commitments stored on-chain

**Pros**: Minimal on-chain storage
**Cons**: Complex proof generation, proof availability concerns, complexity

The next iteration of account compression will likely look similar to this
but the complexity isn't currently necessary.

### reduce and fix hot-set size with a chili peppers-like approach

**Pros**: keeps all data on-chain, no need for users to manually recover old
accounts
**Cons**: doesn't reduce total state size so snapshots remain large and
rent remains high

Chili peppers has other applications but may not be necessary if account
compression can reduce the global state size sufficiently.

### Conclusion

The proposed hash-based approach provides the optimal balance of storage
savings, performance predictability, and implementation complexity.

## Impact

### DApp and Wallet Developers
- include checks for account compression status in program interaction
  workflow
- add instructions to transactions for account recovery when appropriate
- collisions on the `pubkey_hash` are extremely unlikely but not impossible
  due to using 10-bytes. In this case, the user or developer will need to use
  a different address.

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
  hash comparison
- **Atomic operations**: Compression/recovery operations are atomic to
  ensure consistent state across the cluster

### Attack Vectors

- **Hash collision attacks**: collisions on the `data_hash` would allow for
  introducing arbitrary data into the account state. SHA-256 provides
  sufficient collision resistance. Collisions on the `pubkey_hash` are not
  concerning since they're extremely rare and only prevent the new account
  from being created.

## Backwards Compatibility

This feature introduces breaking changes:

### Bank Hash Changes

- **Impact**: Bank hash calculation includes compressed accounts, affecting
  consensus
- **Mitigation**: Feature gate activation ensures all validators adopt
  simultaneously

### Snapshot Format

- **Impact**: New snapshot format with compressed accounts section
- **Mitigation**: Version-aware snapshot loading with backward compatibility
  for old snapshots

### Account Creation Behavior  

- **Impact**: Account creation fails if `SHA_256(pubkey)[0:10]` exists in
  compressed set
- **Mitigation**: if the account corresponding to the pubkey was previously
  compressed it must be recovered rather than recreated. if a collision has
  occured a different pubkey must be used. RPCs and good errors can provide
  the relevant info.
