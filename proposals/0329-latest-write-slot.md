---
simd: '0329'
title: Track Latest Write Slot
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-08-01
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

- Track `latest_write_slot` for each account as part of the account's metadata.
- Each time a transaction that write-locks an account successfully executes and
  is included in a block, update that account's `latest_write_slot` to the
  block's slot. Updates MUST occur after successful execution; aborted/failed
  transactions MUST NOT bump this field.

## Motivation

Solana currently does not persistently record when an account was last
write-locked by an included transaction. This feature is required by
[SIMD-0344](https://github.com/solana-foundation/solana-improvement-documents/pull/344).

Including this ahead of future features (e.g. a new rent system) allows for
a more accurate view on historical account activity when those features are
activated.

## New Terminology

- **Account `latest_write_slot`**: the latest slot in which an account was
  write-locked by a successfully executed transaction that was included in a
  block.

## Detailed Design

### Account Metadata Extension

Account metadata repurposes the deprecated `rent_epoch` field (8 bytes) to store
`latest_write_slot`, encoded as an unsigned little-endian 8-byte integer. This
is a semantic replacement of an existing field; the total size of account
metadata is unchanged.

#### Implementation Details

1. **New Account Creation**: When a new account is created (via system program
   allocation or other means), `latest_write_slot` MUST be set to the current
   slot.

2. **Update on Write-Lock**: For each account write-locked by a transaction
   that successfully executes (status Ok) and is accepted into the block, that
   account's `latest_write_slot` MUST be set to the block's slot. The update
   MUST occur after successful execution in the commit path. Transactions that
   abort or fail during execution or are dropped before commit MUST NOT update
   `latest_write_slot`.

3. **Snapshot Integration**: `latest_write_slot` MUST be included in account
   data when serializing snapshots and MUST be restored when deserializing
   snapshots.

4. **Initialization for Existing Accounts**: For accounts that exist before
   activation of this feature, `latest_write_slot` MUST be initialized to the
   slot of the block that activates the feature.

5. **RPC and Client Exposure**:
   - `latest_write_slot` SHOULD be exposed via RPC in account-returning
     endpoints as a new field named `latestWriteSlot`.
   - For backwards compatibility, the legacy `rentEpoch` field SHOULD remain in
     RPC responses. After feature activation, RPC SHOULD default `rentEpoch` to
     `u64::MAX` for accounts, signaling deprecation of that value in post-
     activation states.

6. **Feature Gate Activation**: This change is guarded by a feature gate. Upon
   activation in a block at slot S, all existing accounts MUST have
   `latest_write_slot` initialized to S when loaded (e.g., from snapshot or
   ledger). New accounts created after activation MUST initialize the field to
   the current slot at creation time.

7. **Forks and Rollbacks**: Since the value is set to the block's slot, normal
    bank forking/rollback naturally reverts/advances the value with the bank's
    state. Replay on the chosen fork MUST yield the same `latest_write_slot`
    values.

8. **Accounts lattice hash update**: the lattice hash of an account must
    include the `latest_write_slot`:

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
      lthash.append( account.latest_write_slot )
      return lthash.fini()
   ```

9. **On-chain ABI exposure**: `latest_write_slot` is runtime metadata only and
   is NOT added to the on-chain `AccountInfo` ABI. It is not exposed to
   programs, and no new syscalls are introduced by this SIMD. Any future
   on-chain exposure (e.g., a syscall or ABI extension) will be proposed via a
   separate SIMD and feature gate.

10. **Simulation semantics**: During `simulateTransaction`/preflight, the
    returned simulated account state SHOULD reflect a virtual bump of
    `latest_write_slot` to the current slot for any accounts that the simulated
    transaction would write-lock and successfully execute.

#### Storage Considerations

No increase in account or snapshot size: the feature reuses the existing 8-byte
`rent_epoch` field to store `latest_write_slot`.

#### Snapshots

- **Pre-activation snapshots (legacy)**: Snapshots produced before feature
  activation contain the deprecated `rent_epoch` value in the reused 8-byte
  field. When loading such a snapshot:
  - If the loaded bank does not yet have the feature active (snapshot slot < S),
    the field continues to be treated as `rent_epoch` until activation.
  - At feature activation slot S, all accounts MUST have `latest_write_slot`
    logically initialized to S. This initialization is part of the
    deterministic state transition and applies uniformly across validators.

- **Post-activation snapshots**: Snapshots produced at or after activation MUST
  serialize `latest_write_slot` in place of `rent_epoch`. Older software that
  lacks the feature will not be able to deserialize these snapshots.

## Alternatives Considered

N/A

## Impact

### Validators

- none: the client software must automatically handle old account formats
  to fill in the right default for `latest_write_slot`.

### Core Contributors

- Provides a simple, in-protocol signal for "latest write activity" per account.
- Enables downstream features such as rent/TTL, pruning/compaction, and
  analytics without additional tracking structures.

## Security Considerations

N/A

## Backwards Compatibility

This change is designed to be backwards compatible:

1. **RPC Compatibility**: Existing RPC calls will continue to work.
   `latest_write_slot` can be added as an additional field in responses without
   breaking existing clients.

2. **Account Structure**: The deprecated `rent_epoch` metadata field (8 bytes)
   is replaced with `latest_write_slot` (8 bytes). Total size and layout of
   account metadata remain unchanged; only the semantic meaning of that field
   changes.

3. **Snapshot Compatibility**:
   1. New snapshots will include `latest_write_slot` as part of the account
   metadata (e.g. in `AccountSharedData`) reusing the `rent_epoch` bytes.
   2. Pre-activation snapshots are readable: the reused 8-byte field is
   interpreted as `rent_epoch` until the feature activates, at which point all
   accounts are initialized to `latest_write_slot = S`.
4. **ABI Compatibility**: No changes are made to the `AccountInfo` ABI; on-chain
   program interfaces remain unchanged.
