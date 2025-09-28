---
simd: '0371'
title: Alpenglow Fast Handoff Marker
authors:
  - ksn6 (Anza)
  - Ashwin Sekar (Anza)
  - Kobi Sliwinski (Anza)
category: Standard
type: Core
status: Review
created: 2025-09-28
feature: TBD
---

## Summary

(This combines and replaces SIMDs 337 and 366.)
Upgrade `BlockMarkerV1` from SIMD-0307 to `BlockMarkerV2`, introducing
`BlockHeader` and `UpdateParent` variants. `BlockHeader` is placed at the
beginning of block payload and indicates the parent of a block. `UpdateParent`
signals that the intended parent is different than initially indicated. This
paves the way for Fast Leader Handoff support in Alpenglow (section 2.7 of
https://www.anza.xyz/alpenglow-1-1), increasing throughput.

## Motivation

SIMD-0326 proposes Alpenglow, a consensus protocol specified in
https://www.anza.xyz/alpenglow-1-1. Section 2.7 in the paper describes fast
leader handoff. This mechanism allows leaders to begin constructing blocks on
tentative parent blocks before the parent is ensured to be correct. In the
common case, this optimization increases throughput, by giving more time to the
leader to stream the block. To ensure correctness, the leader may need to change
the indicated parent while streaming the block.

## New Terminology

- **BlockHeader**: Block marker variant indicating the block's parent. Placed in
  the beginning of block data.
- **UpdateParent**: Block marker variant indicating a changed parent.

## Detailed Design

### Block Marker Version 2

We introduce `BlockMarkerV2` as an extension of the original `BlockMarkerV1`:

```
BlockMarkerV2 Layout:
+---------------------------------------+
| Variant ID                  (1 byte)  |
+---------------------------------------+
| Variant Data Byte Length   (2 bytes)  |
+---------------------------------------+
| Variant Data              (variable)  |
+---------------------------------------+

Variants:
- 0: BlockFooter (inherited from V1)
- 1: BlockHeader (new in V2)
- 2: UpdateParent (new in V2)
```

### Specification

`BlockHeader` contains info about the parent block. `UpdateParent` marker
contains info about the new parent block that the leader switched to as a result
of fast leader handoff. Each variant includes its own version field:

```
VersionedBlockHeader Layout:
+---------------------------------------+
| Version                     (1 byte)  |
+---------------------------------------+
| Payload (version-specific) (variable) |
+---------------------------------------+
```

```
VersionedUpdateParent Layout:
+---------------------------------------+
| Version                     (1 byte)  |
+---------------------------------------+
| Payload (version-specific) (variable) |
+---------------------------------------+
```

For Version 1:

```
BlockHeaderV1 Payload:
+---------------------------------------+
| Parent Slot                (8 bytes)  |
+---------------------------------------+
| Parent Block ID           (32 bytes)  |
+---------------------------------------+

Total payload size: 40 bytes
Total size with version: 41 bytes
```

```
UpdateParentV1 Payload:
+---------------------------------------+
| Parent Slot                (8 bytes)  |
+---------------------------------------+
| Parent Block ID           (32 bytes)  |
+---------------------------------------+

Total payload size: 40 bytes
Total size with version: 41 bytes
```

Fields:

- **Version**: Versioning byte for forward compatibility (u8)
- **Parent Slot**: The slot number of the (new) parent block (u64,
  little-endian)
- **Parent Block ID**: The block ID identifying the (new) parent block (32-byte
  hash). The meaning of this field is left to specification in future
  Alpenglow-related SIMDs.

### `UpdateParent` Code Sample

```rust
/// Always deserializes to the `Current` variant for forward compatibility.
/// ┌─────────────────────────────────────────┐
/// │ Version                      (1 byte)   │
/// ├─────────────────────────────────────────┤
/// │ Update Data               (variable)    │
/// └─────────────────────────────────────────┘
pub enum VersionedUpdateParent {
    V1(UpdateParentV1),
    Current(UpdateParentV1),
}

/// Version 1 parent update data.
/// ┌─────────────────────────────────────────┐
/// │ Parent Slot                  (8 bytes)  │
/// ├─────────────────────────────────────────┤
/// │ Parent Block ID             (32 bytes)  │
/// └─────────────────────────────────────────┘
pub struct UpdateParentV1 {
    pub new_parent_slot: Slot,
    pub new_parent_block_id: Hash,
}
```

### Integration with BlockComponent

The `UpdateParent` marker integrates with the existing `BlockComponent` system:

```rust
/// Version 2 block marker.
/// Supports V1 features plus BlockHeader and UpdateParent.
/// ┌─────────────────────────────────────────┐
/// │ Variant ID                   (1 byte)   │
/// ├─────────────────────────────────────────┤
/// │ Byte Length                  (2 bytes)  │
/// ├─────────────────────────────────────────┤
/// │ Variant Data              (variable)    │
/// └─────────────────────────────────────────┘
///
/// The byte length field indicates the size of the variant data that follows,
/// allowing for proper parsing even if unknown variants are encountered.
pub enum BlockMarkerV2 {
    BlockFooter(VersionedBlockFooter),
    BlockHeader(VersionedBlockHeader),
    UpdateParent(VersionedUpdateParent),
}
```

### Serialization Format

Both `BlockHeader` and `UpdateParent` (variant 1 and 2) have the following
structure while serialized within a `BlockComponent` in the first version:

```
+---------------------------------------+
| Entry Count = 0             (8 bytes) |
+---------------------------------------+
| Marker Version = 2          (2 bytes) |
+---------------------------------------+
| Variant ID = {1, 2}         (1 byte)  |
+---------------------------------------+
| Length = 41                 (2 bytes) |
+---------------------------------------+
| Version = 1                 (1 byte)  |
+---------------------------------------+
| Parent Slot                 (8 bytes) |
+---------------------------------------+
| Parent Block ID            (32 bytes) |
+---------------------------------------+

Total size: 54 bytes per marker
```

### Marker Placement Rules

`BlockHeader` must be placed at the beginning of shred data of the first FEC
set of every block.

At most one `UpdateParent` marker might be placed in a block, at the beginning
of the shred data of one FEC set. Only the first block of a leader window can
include the `UpdateParent` marker.

Otherwise, the given block is invalid.

### `DATA_COMPLETE_SHRED` Placement Rules

Due to the logic of Alpenglow, validators need to be able to process the
`UpdateParent` marker outside of replay, even without ever replaying the initial
parent block. To make this possible, we introduce additional rules for the
`DATA_COMPLETE_SHRED` flag in data shreds. Most implementations already follow
this rule, but it's not yet enforced.

FEC sets in Alpenglow will follow a fixed structure:

- Each FEC set contains exactly 32 data shreds (insufficient data is
  zero-padded)
- Each FEC set contains exactly 32 coding shreds for error correction.

All implementations MUST enforce that the `DATA_COMPLETE_SHRED` flag can only
appear as `true` on the final data shred within an FEC set (index position 31
within the set). Otherwise, the block is invalid.

All implementations MUST enforce that `UpdateParent` marker can only start at
the beginning of an FEC set, AND the previous FEC set's last data shred shows
`DATA_COMPLETE_SHRED` as `true`. Otherwise, the block is invalid.

With these rules, `UpdateParent` marker can be detected at shred ingestion,
without replaying block contents up to this point, like so:

- Check the 0th shred of the next FEC set when `DATA_COMPLETE_SHRED` is `true`
  in the current shred
- Check the current shred when the `DATA_COMPLETE_SHRED` is `true` in the
  previous shred.

See https://github.com/anza-xyz/alpenglow/pull/459 for an example implementation.

**State Transition Rules**:

After Fast Leader Handoff is implemented, state transition will work as follows:

- State changes from transactions before the `UpdateParent` marker are discarded
- The `UpdateParent` marker itself does not modify state
- Post-`UpdateParent` marker transactions are built on the new parent's state
- Notably, despite pre-marker transaction state changes being discarded, all
  pre-marker shred data WILL be included in the constructed block.

## Alternatives Considered

- **Special Transaction Type**.
  Rejected, because of overhead, and difficult and inconvenient implementation.

- **Extended Block Footer**.
  Rejected, because:
  - Difficult to represent the switch point withing the block.
  - The footer appears at the end of the block, delaying the parent switch
    detection.

- **Shred Modification**.
  Rejected, due to significant protocol changes required, backward compatibility
  challenges, complexity and impact on existing shred processing.

## Impact

- **Positive**:
  - Paves the way for fast leader handoff that will increase throughput.

- **Negative**:
  - Slight block size overhead.

## Security Considerations

Upon launching Alpenglow, validators must validate blocks according to the
Alpenglow protocol.

## Backwards Compatibility

Blocks containing `UpdateParent` markers are **not** backward compatible with
validators that do not support Alpenglow. Blocks will not include `UpdateParent`
markers until the Alpenglow feature flag has been activated.

## Reference Implementation

A reference implementation is available here:
https://github.com/anza-xyz/alpenglow/pull/364