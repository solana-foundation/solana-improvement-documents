---
simd: '0340'
title: Validate chained block id
authors:
  - Charles Li
category: Standard
type: Core
status: Review
created: 2025-08-20
feature: TBD
---

## Summary

Specify the protocol for validating chained block ids under both
TowerBFT and Alpenglow.

## Motivation

Block ids are a unique identifier for every block, being the last FEC
set's merkle root (chained merkle root) or rollup merkle root of all FEC
sets (double merkle root). Given a starting block id, every validator
client can validate the canonical linear chain all the way back to a
snapshot using chained / double merkle roots (two notes: 1. see
SIMD-0333 proposal for including a block id in the snapshot manifest. 2.
Alpenglow uses double merkle roots, not chained merkle roots, and will
be discussed later). This linear chain MUST be explicitly validated and
enforced by every client.

Without a canonical linear chain, you don't know if your parent slot is
in fact your actual parent block because slot numbers do not key blocks
uniquely when there is equivocation.

This is important for both TowerBFT and Alpenglow consensus. TowerBFT
and Alpenglow both need this to repair the consensus block if the client
receives the "wrong" block for a slot (ie. not the one the cluster is
converging on).

## Dependencies

This proposal depends on the following accepted proposals:

- **[SIMD-0313]: Drop unchained merkle roots**

    As the new shred format is being sent by all clients, this feature
    deprecates the old shred format.

- **[SIMD-0333]: Serialize block ID in bank into snapshot**

    Currently the block ID (the merkle root of the last FEC set) of the
    snapshot slot is not included in the snapshot Bank Fields. This
    would enforce the snapshot producer to include the block ID in the
    serialized bank.

## New Terminology

N/A

## Detailed Design

In TowerBFT, clients MUST verify the chained merkle root of all FEC sets
in a slot, both intra-slot and inter-slot. This includes all FEC sets
when there are multiple blocks for a slot (equivocation).

For intra-slot CMR verification, every FEC set in a slot MUST contain
in the chained merkle root field the merkle root of its parent FEC set.
The parent FEC set is defined as the FEC set with fec_set_idx - 32 away,
such that every slot begins with fec_set_idx 0 and ends with a
fec_set_idx that is an arbitrary multiple of 32, though as of time of
writing the maximum allowed FEC set idx is 32768 - 32 = 32736.

For inter-slot CMR verification, the first FEC set's chained merkle root
MUST contain the merkle root of the parent slot's last FEC set merkle
root. The parent slot is defined by the `parent_off` field for every
shred in the FEC set. This provides chaining across slot boundaries.

On detection of a chained merkle root conflict, the client MUST mark the
slot dead. A chained merkle root violation can occur at any arbitrary
FEC set in the slot, so a partially replayed slot can be marked dead. If
verification fails intra-slot, the client MUST mark the associated slot
as dead. If the verification fails inter-slot, the client MUST mark the
_child slot_ as dead.

Note that marking dead is at the slot-, not block- or FEC set-, level.
If there are multiple blocks for a given slot (equivocation), and one
block contains an invalid chain but the other a valid chain, any client
that observes the invalid chain MUST mark the slot as dead.

In case the client observes the valid chain, verifies the slot fully,
and later observes the invalid chain, the client MUST not mark the slot
dead but instead ignore the FEC sets with the invalid chain.

After marking dead, clients MUST continue to propagate shreds through
Turbine even after CMR verification failure. This allows downstream
nodes in the Turbine tree to also observe the failure.

In Alpenglow, the design is exactly the same as above, except instead of
chained merkle root verification Alpenglow uses double merkle root
verification. The differences from chained merkle root verification are:

- Intra-slot: at the end of every slot, the leader transmits a double
  merkle root, which is the root of a second merkle tree generated from
  every FEC set in the slot's merkle root. The client MUST verify this
  double merkle root by checking that double merkle root they
  independently calculate from their received FEC sets matches. There is
  no longer a need to examine the parent FEC set (fec_set_idx - 32). If
  any FEC set has an incorrect merkle root, the double merkle root will
  not match and will fail verification, and the associated slot is
  marked dead.

- Inter-slot: the block header contains the parent slot's double merkle
  root. If the block's header does not match the parent slot's double
  merkle root, then the associated slot with the block header (child, in
  this case) is marked dead.

Additionally, Alpenglow clients MUST explicitly vote "skip" on any
slot that fails double merkle root verification.

Finally, it is possible in case of equivocation that the cluster
converges on the slot, despite the client observed an invalid chain,
because there exists an alternate (equivocating) block containing a
valid chain, and an honest majority observed it. Consensus and repair
then work together to resolve the dead block.

In TowerBFT, the client knows the cluster is converging on an alternate
block if the alternate block gets "duplicate confirmed". This notifies
repair, and the validator will repair (and subsequently replay) the
alternate version of the block.

In Alpenglow, the client knows the cluster is converging on an alternate
block if it observes a valid block certificate for the slot. This block
certificate will be used by repair to retrieve (and subsequently replay)
the alternate block.

## Alternatives Considered

N/A

## Impact

Clients will mark slots that don't have a valid chained block id as dead
that they previously would have replayed and voted on.

## Security Considerations

Security is improved because this ensures all clients unequivocally
agree on a canonical linear chain of block ids and defends against
equivocation attacks and related variants.

## Backwards Compatibility

This feature is backwards compatible with the existing chained merkle
shred format. It will be upgraded to use double merkle shreds when Alpenglow
is released.

However, this SIMD will be feature gated. Blocks that previously passed
consensus will now be marked dead. Thus validators will need to
coordinate rollout of this upgrade.
