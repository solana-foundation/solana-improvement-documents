---
simd: '0106'
title: Partitioned Epoch Rewards PDA
authors:
  - Tyera Eulberg
category: Standard
type: Core
status: Draft
created: 2024-01-17
feature: https://github.com/solana-labs/solana/issues/32166
extends: 0015
---

## Summary

Partitioned epoch rewards (SIMD 0015) will split epoch rewards distribution
across multiple blocks at the start of an epoch. This extension of that SIMD
proposes storing data about the rewards calculation and partitions in a sysvar
account at a program-derived address (PDA), which can be queried by clients, or
indeed within the runtime, to performantly identify the partition for the
distribution to any particular address.

## Motivation

When we move to partitioned epoch rewards as described in the original SIMD, the
only way to find the stake or voting rewards for a specific address will be to
iterate through blocks at the beginning of each epoch, inspecting all the
rewards. This is because the runtime does not persist information about how the
rewards were partitioned; in fact, it does not even persist how many blocks the
rewards distribution spans, so there is no way to predict how long it will take
or how far through the epoch to go in finding a particular address.

## Alternatives Considered

An alternative to storing partition data in an on-chain account would be to
record the necessary data in the ledger in some fashion. This could be a
transaction that gets added to the block, a special RewardType, or metadata that
gets stored on the node and then duplicated to long-term storage, like Bigtable.
In fact, it is probably worthwhile to pursue this alternative as well, since it
will enable finding rewards without access to snapshots or the running chain.

## New Terminology

None

## Detailed Design

When partitioned rewards are calculated in the runtime (currently during the
first block of the epoch), the runtime should populate a PDA that stores the
partition data needed to recreate the hasher that returns the partition index
for any address. This data comprises: the number of partitions, the parent
blockhash, and the hasher kind.

The address of this PDA should include the current epoch number (which contains
the distributions) as a little-endian u64, as well as some bytes to prevent
griefing (eg. `b"EpochRewardsPartitionData"`). The owning program should be the
Stake Program.

## Impact

The change in this proposal does increase the number of "forever" accounts that
validators must store by one per epoch. However, the PDAs will be owned by the
stake program, so could be adjusted or closed in the future by a feature-gated
change to the Stake Program. Meanwhile, the change greatly improves the
post-SIMD 0015 situation for clients trying to track stake or voting rewards,
since they can use the data in the PDA to pull the correct partition directly,
instead of scanning an unknown number of blocks.

## Security Considerations

Like traditional sysvars, the new accounts should only be loadable as read-only.
How this would be accomplished depends on the outcome of SIMD 0105.

## Backwards Compatibility

Population of an account each epoch is a consensus-breaking change and must be
activated with a feature gate. Since this is an extension of SIMD 0015, it
should be gated by the same feature id.
