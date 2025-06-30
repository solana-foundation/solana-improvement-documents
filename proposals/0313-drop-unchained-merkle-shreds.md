---
simd: '0313'
title: Drop unchained merkle shreds
authors:
  - Ashwin Sekar
category: Standard
type: Core
status: Review
created: 2025-06-30
feature:
  - 5KLGJSASDVxKPjLCDWNtnABLpZjsQSrYZ8HKwcEdAMC8
  - https://github.com/anza-xyz/feature-gate-tracker/issues/80
---

## Summary

As the new shred format is being sent by all clients, this feature deprecates
the old shred format.

## Motivation

It is inconvinient to support both formats, and upcoming consensus changes (alpenglow)
expect all shreds to be of the chained variety.

## New Terminology

N/A

## Detailed Design

The chained merkle shred variant, adds an additional field to Data and Coding shreds.
It adds the Merkle root of the previous erasure batch after the data buffer or
erasure coded shard respectively, before the merkle proof:

```
// Layout: {common, data} headers | data buffer
//     | [Merkle root of the previous erasure batch if chained]
//     | Merkle proof
//     | [Retransmitter's signature if resigned]
// The slice past signature till the end of the data buffer is erasure coded.
// The slice past signature and before the merkle proof is hashed to generate
// the Merkle tree. The root of the Merkle tree is signed.

// Layout: {common, data} headers | data buffer
//     | [Merkle root of the previous erasure batch if chained]
//     | Merkle proof
//     | [Retransmitter's signature if resigned]
// The slice past signature till the end of the data buffer is erasure coded.
// The slice past signature and before the merkle proof is hashed to generate
// the Merkle tree. The root of the Merkle tree is signed.
```

Additionally in the common shred header, the first 4 bits of the shred variant
field are reserved to specify the shred variant. The second bit indicates if this
shred is of the chained merkle shred variant.

If `drop_unchained_merkle_shreds: 5KLGJSASDVxKPjLCDWNtnABLpZjsQSrYZ8HKwcEdAMC8`
is active, then any shred with the second bit of the shred variant as zero will
be dropped on ingest.

## Alternatives Considered

None

## Impact

Any clients still producing blocks using the old shred format will have their
shreds ignored on ingest.

## Security Considerations

None

## Backwards Compatibility

This feature is not backwards compatible.
