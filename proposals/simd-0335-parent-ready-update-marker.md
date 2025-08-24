---
simd: '0337'
title: ParentReadyUpdate Block Marker
authors:
  - ksn6 (Anza)
category: Standard
type: Core
status: Review
created: 2025-08-13
feature: TBD
---

## Summary

This SIMD proposes upgrading `BlockMarkerV1`, introduced in SIMD-0307, to
`BlockMarkerV2` to support fast leader handover in the Alpenglow consensus
protocol. In particular, `BlockMarkerV2` includes a new `ParentReadyUpdate`
variant to support fast leader handover. This marker signals when a leader
has switched to a different parent block during fast leader handover
due to a `ParentReady` event, providing critical metadata for consensus
validation and block verification.

## Motivation

The Alpenglow consensus protocol, as specified in SIMD-0326, introduces
fast leader handover as a key performance optimization. This mechanism
allows leaders to begin constructing blocks on tentative parent blocks before
the actual parent is fully finalized through the notarization process. This
optimization significantly reduces block production latency by enabling
parallel processing of block construction and parent finalization.

However, during fast leader handover, a leader may need to switch from
their initially assumed parent to a different parent block when a
`ParentReady` event occurs. This event indicates that:

1. All necessary skip certificates have been collected for intervening slots
2. A `NotarizeFallback` certificate exists for a valid parent block
3. The leader must switch to this newly ready parent (as specified in
   Algorithm 3, Line 7 of the
   [Alpenglow whitepaper](https://www.anza.xyz/alpenglow-1-1))


Currently, there is no standardized mechanism to signal these parent switches
within the block structure. As such, without the capability that this SIMD
details, leader validators cannot indicate that a parent switch has occurred
during fast leader handover.

This SIMD addresses these issues by leveraging the extensible `BlockMarker`
framework introduced in SIMD-0307 to add a dedicated marker for parent switch
events.

## New Terminology

- **Block Marker**: A chunk of structured non-transaction data that can be
  placed before, after, or in-between entry batches in a block.
- **Fast Leader Handover**: A technique where leaders begin constructing
  blocks on a tentative parent before the actual parent is finalized
- **ParentReady Event**: A consensus event indicating that a block has met
  all requirements to serve as a parent for new block production
- **Parent Switch**: The action of changing from one parent block to another
  during fast leader handover
- **ParentReadyUpdate**: A block marker that records when and to which parent
  a switch occurred during fast leader handover

## Detailed Design

This proposal extends the block marker system introduced in SIMD-0307 by
adding a new variant specifically for Alpenglow's fast leader handover
requirements.

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
- 1: ParentReadyUpdate (new in V2)
```

### ParentReadyUpdate Specification

The `ParentReadyUpdate` marker contains information about the new parent block
that the leader switched to during fast leader handover:

```
ParentReadyUpdateV1 Layout:
+---------------------------------------+
| Parent Slot                (8 bytes)  |
+---------------------------------------+
| Parent Block ID           (32 bytes)  |
+---------------------------------------+

Total size: 40 bytes
```

Fields:

- **Parent Slot**: The slot number of the new parent block (u64,
  little-endian)
- **Parent Block ID**: The block ID identifying the new parent block (32-byte
  hash)

### Versioned Structure

Following the pattern established in SIMD-0307, we use a versioned approach
for future extensibility. We illustrate code samples below in Rust, although
other languages may be used:

```rust
pub enum VersionedParentReadyUpdate {
    V1(ParentReadyUpdateV1),
    Current(ParentReadyUpdateV1),
}

pub struct ParentReadyUpdateV1 {
    pub new_parent_slot: Slot,
    pub new_parent_block_id: Hash,
}
```

### Integration with BlockComponent

The `ParentReadyUpdate` marker integrates with the existing `BlockComponent` system:

```rust
pub enum BlockMarkerV2 {
    BlockFooter(VersionedBlockFooter),
    ParentReadyUpdate(VersionedParentReadyUpdate),
}
```

### Serialization Format

When serialized within a `BlockComponent`, the complete structure is:

```
+---------------------------------------+
| Entry Count = 0             (8 bytes) |
+---------------------------------------+
| Marker Version = 2          (2 bytes) |
+---------------------------------------+
| Variant ID = 1              (1 byte)  |
+---------------------------------------+
| Length = 40                 (2 bytes) |
+---------------------------------------+
| Parent Slot                 (8 bytes) |
+---------------------------------------+
| Parent Block ID            (32 bytes) |
+---------------------------------------+

Total overhead: 53 bytes per marker
```

## Alternatives Considered

- **No Explicit Marking / Do Nothing**.
  Relying on implicit detection of parent switches was considered but rejected
  because of:
  - Ambiguity in determining whether a parent switch occurred vs. a duplicate
    block was received
  - Inability to validate leader behavior
  - Lack of auditability for fast leader handover decisions

- **Special Transaction Type**.
  Using a special transaction to signal parent switches was considered but
  rejected because:
  - Higher overhead due to transaction processing requirements
  - Having a transaction "undo" the state transitions induced by other
    transactions is ill-advised

- **Extended Block Footer**.
  Extending the block footer to include parent switch information was
  considered but is inadequate because:
  - Parent switches occur during block production, not at the end
  - The footer appears only at block completion
  - Cannot accurately represent the switch point within the block

- **Shred Modification**.
  Modifying the shred structure to include parent switch metadata was
  considered but rejected due to:
  - Significant protocol changes required
  - Backward compatibility challenges
  - Complexity of implementation
  - Impact on existing shred processing infrastructure

## Impact

- **Positive**:
  - Enables full implementation of Alpenglow's fast leader handover
  - Fast leader handover, upon implementation, reduces block production
    latency
  - Minimal space overhead (53 bytes per switch event)

- **Negative**:
  - Slight increase in block size when parent switches occur

## Security Considerations

The introduction of this marker does not introduce any new security risks,
until Alpenglow itself is fully implemented and launched.

## Backwards Compatibility

This SIMD requires SIMD-0307 to be implemented first, as it depends on the
`BlockMarker` framework.

Blocks containing `ParentReadyUpdate` markers are **not** backward compatible
with validators that do not support Alpenglow. Blocks will not disseminate
`ParentReadyUpdate` markers until the Alpenglow feature flag has been
activated.

## Reference Implementation

A reference implementation is available here: https://github.com/anza-xyz/alpenglow/pull/364

This implementation includes:

- Full `BlockComponent` enumeration with marker support
- Serialization/deserialization logic
