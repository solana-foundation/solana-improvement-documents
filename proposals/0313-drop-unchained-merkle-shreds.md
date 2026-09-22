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

It is inconvenient to support both formats, and upcoming consensus changes (alpenglow)
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

// Layout: {common, coding} headers | erasure coded shard
//     | [Merkle root of the previous erasure batch if chained]
//     | Merkle proof
//     | [Retransmitter's signature if resigned]
// The slice past signature and before the merkle proof is hashed to generate
// the Merkle tree. The root of the Merkle tree is signed.
```

Additionally in the common shred header, the first 4 bits of the shred variant
field are reserved to specify the shred variant. Unchained Merkle shreds use
`0b0100` (coding) and `0b1000` (data); chained Merkle shreds use `0b0110`
(coding) and `0b1001` (data), or `0b0111` and `0b1011` when resigned.

If `drop_unchained_merkle_shreds: 5KLGJSASDVxKPjLCDWNtnABLpZjsQSrYZ8HKwcEdAMC8`
is active, then any shred with an unchained variant (`0b0100` or `0b1000`) will
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
