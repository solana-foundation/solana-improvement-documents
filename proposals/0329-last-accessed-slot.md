---
simd: '0329'
title: Track Last Accessed Slot
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-08-01
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

- Track `last_accessed_slot` for each account as part of the account's metadata.
- Each time a transaction that write-locks an account is included in a block,
  update that account's `last_accessed_slot` to the block's slot.

## Motivation

Solana currently does not persistently record when an account was last
write-locked by an included transaction. Capturing the most recent
write-lock slot per account enables downstream features such as rent or
TTL-style policies operational tooling. This SIMD introduces a single metadata
field with simple update semantics to make that signal universally available
without introducing additional tracking structures.

Including this ahead of future features (e.g. a new rent system) allows for
a more accurate view on historical account activity when those features are
activated.

## New Terminology

- **Account `last_accessed_slot`**: the latest slot in which an account was
  write-locked by a transaction that was included in a block.

## Detailed Design

### Account Metadata Extension

Account metadata includes an additional `last_accessed_slot` field encoded as an
unsigned little-endian 8-byte integer.

#### Implementation Details

1. **New Account Creation**: When a new account is created (via system program
   allocation or other means), `last_accessed_slot` MUST be set to the current
   slot.

2. **Update on Write-Lock**: Each time a transaction that write-locks an
   account is included in a block, that account's `last_accessed_slot` MUST be
   set to the block's slot.

3. **Snapshot Integration**: `last_accessed_slot` MUST be included in account
   data when serializing snapshots and MUST be restored when deserializing
   snapshots.

4. **Initialization for Existing Accounts**: For accounts that exist before
   activation of this feature, `last_accessed_slot` MUST be initialized to the
   slot of the block that activates the feature.

5. **RPC and Client Exposure**: `last_accessed_slot` SHOULD be available
   through relevant RPC endpoints that return account information, allowing
   clients to access this metadata.

6. **Feature Gate Activation**: This change is guarded by a feature gate. Upon
   activation in a block at slot S, all existing accounts MUST have
   `last_accessed_slot` initialized to S when loaded (e.g., from snapshot or
   ledger). New accounts created after activation MUST initialize the field to
   the current slot at creation time.

7. **Forks and Rollbacks**: Since the value is set to the block's slot, normal
    bank forking/rollback naturally reverts/advances the value with the bank's
    state. Replay on the chosen fork MUST yield the same `last_accessed_slot`
    values.

8. **Accounts lattice hash update**: the lattice hash of an account must
    include the `last_accessed_slot`:

    ```
   lthash(account) :=
   if account.lamports == 0:
      return 00..00
   else:
      lthash.init()
      lthash.append( account.lamports )
      lthash.append( account.data )
      lthash.append( account.is_executable )
      lthash.append( account.owner )
      lthash.append( account.pubkey )
      lthash.append( account.last_accessed_slot )
      return lthash.fini()
   ```

9. **On-chain ABI exposure**: `last_accessed_slot` is runtime metadata only and
   is NOT added to the on-chain `AccountInfo` ABI. It is not exposed to
   programs, and no new syscalls are introduced by this SIMD. Any future
   on-chain exposure (e.g., a syscall or ABI extension) will be proposed via a
   separate SIMD and feature gate.

#### Storage Considerations

The additional 8 bytes per account may increase snapshot size if insufficient
unused account metadata bytes are available. Currently, there are enough unused
bytes (due to padding and deprecated fields) so this specific change shouldn't
increase state size.

## Alternatives Considered

N/A

## Impact

### Validators

- none: the client software must automatically handle old account formats
  to fill in the right default for `last_accessed_slot`.

### Core Contributors

- Provides a simple, in-protocol signal for "last write activity" per account.
- Enables downstream features such as rent/TTL, pruning/compaction, and
  analytics without additional tracking structures.

## Security Considerations

N/A

## Backwards Compatibility

This change is designed to be backwards compatible:

1. **RPC Compatibility**: Existing RPC calls will continue to work.
   `last_accessed_slot` can be added as an additional field in responses without
   breaking existing clients.

2. **Account Structure**: `last_accessed_slot` is new metadata and does not
   modify existing account data or behavior. Unused bytes and bytes from
   deprecated fields will be used to store this value.

3. **Snapshot Compatibility**:
   1. New snapshots will include `last_accessed_slot` as part of the account
   metadata (e.g. in `AccountSharedData`). If the snapshot was created before
   feature activation, `last_accessed_slot` is initialized to the activation
   slot when loading.
4. **ABI Compatibility**: No changes are made to the `AccountInfo` ABI; on-chain
   program interfaces remain unchanged.
