---
simd: '0057'
title: Turbine for Duplicate Block Prevention
authors:
  - Carl Lin
  - Ashwin Sekar
category: Standard
type: Core
status: Draft
created: 2023-10-11
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Duplicate block handling is slow and error prone when different validators see
different versions of the block

## Motivation

In a situation where a leader generates two different blocks for a slot, either:

1) all the validators get the same version of the block.
2) the super majority gets a mixture of shreds from the different versions of the
   block and mark it dead during replay.
3) the network is split and participants get different replayable versions of the
   block.

This proposal attempts to maximize the chance of situations (1) and (2).

## Alternatives Considered

Not applicable

## New Terminology

None, however this proposal assumes an understanding of shreds and turbine:
https://github.com/solana-foundation/specs/blob/main/p2p/shred.md
https://docs.solana.com/cluster/turbine-block-propagation

## Detailed Design

With the introduction of Merkle shreds, each shred is now uniquely attributable
to the FEC set to which it belongs. This means that given an FEC set of minimum
32 shreds, a leader cannot create an entirely new FEC set by just modifying the
last shred, because the `witness` in that last shred disambiguates which FEC set
it belongs to.

This means that in order for a leader to force validators `A` and `B` to ingest
a separate version `N` and `N'` of a block, they must at a minimum create and
propagate two completely different versions of an FEC set. Given the smallest
FEC set of 32 shreds, this means that 32 shreds from one version must arrive to
validator `A`, and 32 completely different shreds from the other version must
arrive to validator `B`.

We aim to make this process as hard as possible by leveraging the randomness of
each shred's traversal through turbine via the following set of changes:

1. Lock down shred propagation so that validators only accept shred `X` if it
arrives from the correct ancestor in the turbine tree for that shred `X`. There
are a few downstream effects of this:

 - In repair, a validator `V` can no longer repair shred `X` from anybody other
 than the singular ancestor `Y` that was responsible for delivering shred `X` to
 `V` in the turbine tree.
 - Validators need to be able to repair erasure shreds, whereas they can only
repair data shreds today. This is because now the set of repair peers is locked,

then if validator `V`'s ancestor `Y` for shred `X` is down, then shred `X` is
unrecoverable. Without being able to repair a backup erasure shred, this would
mean validator `X` could never recover this block

2. If a validator received shred `S` for a block, and then another version of
that shred `S`' for the same block, it will propagate the witness of both of
those shreds so that everyone in the turbine tree sees the duplicate proof. This
makes it harder for leaders to split the network into groups that see a block is
duplicate and groups that don't.

Note these duplicate proofs still need to gossiped because it's not guaranteed
duplicate shreds will propagate to everyone if there's a network partition, or
a colluding malicious root node in turbine. For instance, assuming 1 malicious
root node `X`, `X` can forward one version of the shred to one specific
validator `Y` only, and then only descendants of validator `Y` would possibly
see a duplicate proof when the other canonical version of the shred is
broadcasted.

3. The last FEC set is unique in that it can have less than 32 data shreds.
In order to account for the last FEC set potentially having a 1:32 split of
data to coding shreds, we enforce that validators must see at least half the
block before voting on the block, *even if they received all the data shreds for
that block*. This guarantees leaders cannot just change the one data shred to
generate two completely different, yet playable versions of the block

## Impact

The network will be more resilient against duplicate blocks

## Security Considerations

Not applicable

## Backwards Compatibility

Rollout will happen in stages, as this proposal depends on QUIC turbine

Tentative schedule:
Prevention:

1) Merkle shreds (rolled out)
2) Turbine/Repair features

  - Coding shreds repair
  - Propagate duplicate proofs through turbine
  - 1/2 Shreds threshold for voting (feature flag)

3) QUIC turbine
4) Lock down turbine tree (feature flag and opt-out cli arg for shred forwarders)
