---
title: Epoch Reward Calculation
---

## Problem

Calculating and distributing rewards at one slot on the epoch boundary is not
scalable. With increasing number of stake accounts and vote accounts on
validator nodes, the block time at epoch boundary can last as long as 20
seconds. Such long block time degrades the network performance and becomes a
potential vulnerability of the network.

To solve this problem and speed up the reward time at epoch boundary, an earlier
approach [Partitioned Inflationary Rewards
Distribution](https://github.com/solana-labs/solana/blob/master/docs/src/proposals/partitioned-inflationary-rewards-distribution.md)
is proposed. Let's call this approach `V1`. A prototype of the `V1` proposal is
implemented. While `V1` works, it, however, has two main drawbacks:

1. A restriction is enforced to prevent updates to all stake accounts and vote
accounts during reward interval. In `V1` approach, the stake accounts and vote
accounts are locked during reward interval. All stake account's manipulation
such as delegate, redelegate are not allowed during reward interval.

1. There are no on-chain proof for rewards distribution at the epoch. In `V1`
approach, the rewards distribution is happening inside bank runtime. There is no
on-chian records for the rewards distribution. The rewards distributions are in
a black-box.

Therefore, we are proposing another way, `V2`, to handle epoch reward. The `V2`
approach will completely decouple epoch reward into two parts: (1) reward
calculation and (2) reward distribution.

In this proposal, we will discuss "reward calculation", which will solve the
first problem. A follow-up proposal will discuss about "reward distribution",
which will solve the second problem.

## Proposed Solutions

Instead of computing the rewards at the first slot of epoch boundary, the reward
computation is moved to a background service.

It is very similar to the `V1` proposal, a separate service,
"EpochRewardCalculationService" will be created. The service will listen to a
channel for any incoming rewards calculation requests, and perform the
calculation for the rewards.

The main difference is that instead of sharing the `StakeCache` with the runtime
during reward computation. The `StakeCache` is cloned at slot-height 0 in the
new epoch. In this way, we don't need to lock stake accounts and vote
accounts to prevent updates during the reward calculation.

And the reward are computed for the next `N` slot-hight. At the end of `Nth`
slot-hight, the reward computation result should be available (the runtime will
wait till the computation is finished before processing any blocks with height
greater than `N`).

To make sure that we have a consistent proof of the epoch reward, a merkle tree
will be computed from the epoch rewards. The root of the merkle tree and total
rewards will be recorded on the network though a new system account `EpochRewardProof`
This `EpochRewardProof` account maintains a vector of entries of `(epoch, (reward_amount,
root_hash))`.

"EpochRewardProof" will be used in later for epoch reward distribution. See "Epoch
Reward Distribution" for more details.

### Challenges and Discussions

1. How hard and how expensive are to clone the stake cache? And what's the
   performance overhead for this?

1. What's the overhead to compute the merkle tree?

1. What's the associated complexity to introduce the new `EpochRewardProof`
   system variable?

1. How to adapt the current epoch reward and inflation related RPC methods for
   new way of computing epoch rewards? What kind of new RPC APIs to provide for
   to access the `EpochRewardProof`?

1. How to handle validator restart during reward calculation? By saving and
   loading stake_cache in snapshot? What's the disk footprint for storing
   stake_cache in snapshot archives?
