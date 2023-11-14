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

In a situation where a leader generates two different blocks for a slot, ideally
either all the validators get the same version of the block, or they all see a
mix of the different versions of the block and mark it dead during replay. This
removes the complicated process of reaching consensus on which version of the
block needs to be stored.

## Alternatives Considered

1. Storing all or some 'n' versions of the block - This can be DOS'd if a
malicious leader generates a bunch of different versions of a block, or
selectively sends some versions to specific validators.

2. Running separate consensus mechanism on each duplicate block - Very
complicated and relies on detection of the duplicate block

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

### Duplicate block resolution

Against a powerful adversary, the preventative measures outlined above can be
circumvented. Namely an adversary that controls a large percentage (< 33%) of
stake and has the ability to create and straddle network partitions can
circumvent the measures by isolating honest nodes in partitions.
Within the partition the adversaries can propagate a single version of the
block, nullifying the effects of the duplicate witness proof.

In the worse case we can assume that the adversary controls 33% of the network
stake. By utilizing this stake, they can attack honest nodes by creating network
partitions. In a turbine setup with offline nodes and malicious stake
communicating through side channel, simulations show that 1% of honest nodes can
receive a block given that at least 15% honest nodes are in the partition. [1]

Percentage online is the number of total stake online in the partition. These
simulations were run with 33% of that stake being malicious. Malicious nodes
communicate through side channel to receive the block, and therefore will always
propagate shreds to their children, regardless of whether their parent sent them
the shred.

The simulation was run with 2 different stake weight distributions, an equal
distribution where each validator had the same amount of stake, and a Mainnet
distribution where the number of validators and stake weights directly mapped
to the mainnet beta distribution as of Sept 14th 2023.

Median stake recovered with 33% malicious, 10K trials
| Percentage online | Equal stake | Mainnet stake |
| ----|-----|-----------|
| 33% | 33% | 33%       |
| 40% | 33% | 33%       |
| 45% | 33.3% | 33.09%  |
| 46% | 33.4% | 33.46%  |
| 47% | 33.54% | 33.58% |
| 48% | 33.71% | 34.78% |
| 49% | 33.97% | 36.21% |
| 50% | 34.28% | 39.93% |
| 51% | 34.70% | 42.13% |
| 52% | 35.09% | 43.42% |
| 53% | 35.85% | 45.23% |
| 54% | 36.88% | 46.42% |
| 55% | 37.96% | 47.95% |
| 60% | 48.95% | 55.51% |
| 66% | 64.05% | 64.08% |
| 75% | 74.98% | 74.59% |

Given this we can conclude that there will be at most 5 versions of a block that
can reach a 34% vote threshold, even against the most powerful adversaries, as
there needs to be a non overlapping 15% honest nodes in each partition. [2]

To solve this case we can store up to 5 duplicate forks as normal forks, and
perform normal fork choice on them:

- Allow blockstore to store up to 5 versions of a block.
- Only one of these versions can be populated by turbine. The remaining 4
  versions are only for repair.
- If a version of this slot reaches the 34% vote threshold, attempt to repair
that block. This inherently cannot be from a turbine parent, so it must relax
the constraint from the prevention design.
- From this point on, we treat the fork as normal in fork choice. This requires
that the remaining parts of consensus operate on (Slot, Hash) ids, and that
switching proofs allow stake on the same slot, but different hashes.
- Include the same duplicate witness proofs from the prevention design, and only
vote on blocks that we have not received a proof for, or that have reached the
threshold.

In order to accurately track the threshold, it might be prudent to tally vote
txs from dead blocks as well, in the case gossip is experiencing problems.
Alternatively/Additionally consider some form of ancestory information in votes
[3] to ensure that the vote threshold is viewed. This might be a necessity in
double duplicate split situations where the initial duplicate block is not voted
on.

## Impact

The network will be more resilient against duplicate blocks

## Security Considerations

Not applicable

## Backwards Compatibility

Rollout will happen in stages, prevention cannot be turned on until QUIC turbine.
Resolution can run in tandem with duplicate block consensus v1, and full migration
will be the final step.

Tentative schedule:

Prevention:

1) Merkle shreds (rolled out)
2) Turbine/Repair features

  - Coding shreds repair
  - Propagate duplicate proofs through turbine
  - 1/2 Shreds threshold for voting (feature flag)

3) QUIC turbine
4) Lock down turbine tree (feature flag and opt-out cli arg for jito)

Resolution:

1) Merkle shreds (rolled out)
2) Blockstore/AccountsDb features

  - Duplicate proofs for merkle shreds
  - Store up to 5 versions in blockstore (feature flag for column migration)
  - Store epoch's worth of slot hashes in accountsdb (feature flag)

3) Consensus changes

  - Targetted duplicate block repair
  - Voting checks and 34% repair (feature flag)

4) Migration

  - Unplug DuplicateConfirmed
  - Unplug Ancestor Hashes Service
  - Unplug Popular Pruned

## References

[1] Equal stake weight simulation
    `https://github.com/AshwinSekar/turbine-simulation/blob/master/src/main.rs`
    uses a 10,000 node network with equal stake and shred recovery. Mainnet
    stake weight simulation
    `https://github.com/AshwinSekar/solana/commits/turbine-simulation` mimics
    the exact node count and stake distribution of mainnet and does not perform
    shred recovery.

[2] Section 4
`https://github.com/AshwinSekar/turbine-simulation/blob/master/Turbine_Merkle_Shred_analysis.pdf`

[3] Block Ancestors Proposal
`https://github.com/solana-labs/solana/pull/19194/files`
