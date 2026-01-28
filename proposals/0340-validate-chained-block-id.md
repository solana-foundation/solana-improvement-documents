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

Verify chained merkle roots across slot boundaries.

## Motivation

Currently it is not required to verify that a FEC set merkle root chains
correctly across slot boundaries. Consensus can converge on a block even
if the first FEC set's chained merkle root is invalid ie. does not chain
off the parent block's last FEC set merkle root (the block id).

This is a problem because chained merkle roots should validate an entire
ancestry, so that you have a canonical linear chain (all the way back to
the snapshot slot - see also SIMD-0333 proposal for including a block_id
in the snapshot manifest). Otherwise, you don't know if your parent slot
based on `slot - parent_off` is in fact your actual parent block because
slot numbers do not key blocks uniquely when there is equivocation.

This is important for both TowerBFT and Alpenglow consensus.  Alpenglow
in particular will need this to repair the alternate version of a block
when the parent slot / parent block_id mismatch is due to equivocation.

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

Verify the chained merkle root of the first FEC set in a block correctly
matches the merkle root of the last FEC set of the parent slot (based on
the slot and parent_off field in every shred in the FEC set).

Do not attempt to replay a child block off a parent block unless both
the parent slot (ie. `slot - parent_off`) matches the first FEC's slot
and the parent block_id matches the first FEC's chained merkle root.

However, client implementations should continue to propagate shreds
through Turbine even after observing the chaining violation. This allows
downstream nodes in the Turbine tree to also observe the violation. As
described below, the chaining violation should instead be resolved
through repair and consensus.

In TowerBFT, if they do not match, mark the child block (ie. block with
the first FEC set) as dead. Marking as dead notifies repair, and in case
the rest of the cluster converges on the block, the validator will
repair the consensus alternate version of the block after marking the
initial version dead.

In Alpenglow, it is not necessary to mark the block as dead. Instead,
the validator will vote "skip". In case the rest of the cluster
generates a certificate for an alternate version of the block (due to
equivocation) then the certificate will notify repair which retrieves
the alternate block.

Note also that Alpenglow deprecates chained merkle shreds in favor of a
double merkle shred + block header scheme to solve this problem. The
block header uniquely points a parent, similar to what this SIMD
proposes be done with chained merkle shreds as they exist today.

Clients should refer and adhere to the Alpenglow specification if
implementing Alpenglow consensus.

## Alternatives Considered

N/A

## Impact

Clients will mark blocks that don't properly chain to their parent as
dead that they previously would have replayed.

## Security Considerations

Security is improved because of enhanced equivocation protection.

## Backwards Compatibility

This feature is backwards compatible with the existing chained merkle
shred format. It will be deprecated with the rollout of Alpenglow
(together with chained merkle shreds generally).

However, this SIMD should be feature gated. Blocks with invalid chained
merkle roots to parent blocks previously could have been rooted by
consensus but now will be marked as dead. Thus validators will need to
coordinate rollout of this upgrade.
