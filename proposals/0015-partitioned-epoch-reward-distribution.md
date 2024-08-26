---
simd: '0015'
title: Partitioned Epoch Rewards Distribution
authors:
  - Haoran Yi (Solana Labs)
category: Standard
type: Core
status: Withdrawn
created: 2023-03-02
feature: N/A
superseded-by: '0118'
---

## Summary

A new way to distribute epoch rewards across multiple blocks is proposed to
address the current performance problems associated with epoch reward
distribution in a first block of a new epoch.

## Motivation

The distribution of epoch rewards at the start block of an epoch becomes a
significant bottleneck due to the rising number of stake accounts and voting
nodes on the network.

To address this bottleneck, we propose a new approach for distributing the
epoch rewards over multiple blocks.

## New Terminology

   - rewards calculation: calculate the epoch rewards for all active stake
     accounts

   - rewards distribution: distribute the epoch rewards for the active stake
     accounts

## Alternatives Considered

We have discussed the following alternative approaches.

1. Simply set a threshold on stake balance, and limit the epoch rewards to the
   accounts with stake balance above the threshold. This will effectively
   reduce the number of stake rewards to be distributed, and reduce reward
   distribution time at the epoch boundary. However, this will impact stake
   accounts with small balance. To receive rewards, the small stake accounts
   will be forced to join stake pools, which some may be hesitant to do.

2. Handle reward distributions through transactions with specialized native
   reward programs. While this approach is optimal, it requires significant
   modifications and doesn't align with the current reward code.

3. A completely asynchronous epoch rewards calculation and distribution, in
   which both reward computation and rewards distribution are asynchronous.
   This is the most general approach. However, it is also the most complex
   approach. The reward computation states would have to be maintained across
   multiple blocks. The transition between the reward calculation and reward
   distribution would need to be synchronized across the cluster. And cluster
   restart during reward computation would have to be handled specially.

4. An other approach is similar to the current proposal with additional
   per-block reward reserve sysvar accounts. Those sysvar accounts are
   introduced to track and verify the rewards distributed per block. The
   per-block reward reserve sysvar accounts add additional checks and safety
   for reward distribution. However, they also add addition cost to block
   processing, especially for the first block in the epoch. The first block is
   already computationally heavily - it is responsible for processing all the
   reward computation. The additional cost of those sysvars puts more burden
   onto that block and hurt the timing for it.

## Detailed Design

The major bottleneck for epoch reward distribution is to distribute rewards to
stake accounts. At the time of writing, there are approximately 550K active
stake accounts and 1.5K vote accounts on Solana Mainnet Beta. Given the
relatively small number of vote accounts, it makes sense to keep vote rewards
distribution mechanism unchanged. They can still be distributed efficiently at
the first block of the epoch boundary. This reduces the impact of rewards for
vote account and also simplifies the overall changes. It also lets us focus on
solving the primary bottleneck - Stake Rewards. Only Stake rewards are going to
be distributed out over multiple blocks.

In the new stake rewards distribution approach, we will separate the
computation of rewards from the actual distribution of rewards at the epoch
boundary by dividing the process into two distinct phases:

   1. rewards calculation phase - during which the epoch rewards for all
      activate stake accounts are computed and distribution chunks are scheduled.

   1. rewards distribution phase - during which the calculated epoch rewards
      for the active stake accounts are distributed.

To help maintain the total capital balance and track/verify the reward
distribution during rewarding phases, a new sysvar account, `EpochRewards`, is
proposed. The `EpochRewards` sysvar holds the balance of the rewards that are
pending for distribution.


### Rewards Calculation

Reward calculation phase computes all the rewards that need to be distributed
for the active stake accounts, and partitions the reward into a number of
chunks for distribution in the next phase.

Currently, on Solana Mainnet Beta with ~550K active stake accounts, it shows
that epoch reward calculation takes around 10 seconds on average. This will
make it impossible to perform rewards computation synchronous within one block.

However, there are quite a few promising optimizations that can cut down the
reward computation time. An experiment for reward calculation optimization
(https://github.com/solana-labs/solana/pull/31193) showed that we can cut the
reward calculation time plus vote reward distribution time to around 1s. This
makes synchronous reward computation and asynchronous reward distribution a
feasible approach. We also believe that there is still more rooms for more
optimization to further cut down the above timing.

Therefore, the following design is based on the above optimization. The
reward calculation will be performed at the first block of the epoch. Once the
full rewards are calculated, the rewards will be partitioned into distribution
chunks stored in the bank, which will then be distributed during the `reward
distribution` phase.

To ensure that each block distributes a subset of the rewards in a
deterministic manner for the current epoch, while also randomizing the
distribution across different epochs, the partitioning of all rewards will be
done as follows.

The reward results are sorted by Stake Account address, and randomly shuffled
with a fast `rng` seeded by current epoch. The shuffled reward results are then
divided into `M` chunks. This process will ensure that the reward distribution
is deterministic for the current epoch, while also introducing a degree of
randomness between epochs.

To minimize the impact on block processing time during the reward distribution
phase, a total of 4,096 accounts will be distributed per block. The total
number of blocks needed to distributed rewards is given by the following
formula to avoid using floating point value arithmetic:

```
M = ((4096 - 1)+num_stake_accounts)/4096
```

### `EpochRewards` Sysvar Account

`EpochRewards` sysvar account records the total rewards and the amount of
distributed rewards for the current epoch internally. And the account balance
reflects the amount of pending rewards to distribute.

The layout of `EpochRewards` sysvar is shown in the following pseudo code.

```
struct RewardRewards{
   // total rewards for the current epoch, in lamports
   total_rewards: u64,

   // distributed rewards for  the current epoch, in lamports
   distributed_rewards: u64,

   // distribution of all staking rewards for the current
   // epoch will be completed before this block height
   distribution_complete_block_height: u64,
}
```

The `EpochRewards` sysvar is created at the start of the first block of the
epoch (before any transactions are processed), as both the total epoch rewards
and vote account rewards become available at this time. The
`distributed_reward_in_lamport` field is updated per reward distribution for
each block in the reward distribution phase.

Once all rewards have been distributed, the balance of the `EpochRewards`
account MUST be reduced to `0` (or something has gone wrong). For safety, any
extra lamports in `EpochRewards` accounts will be burned after reward
distribution phase, and the sysvar account will be deleted. Because of the
lifetime of `EpochRewards` sysvar coincides with the reward distribution
interval, user can explicitly query the existence of this sysvar to determine
whether a block is in reward interval. Therefore, no new RPC method for reward
interval is needed.

### Reward Distribution

Reward distribution phase happens after reward computation phase, which starts
after the first block in the epoch for this proposal. It lasts for `M` blocks.
Each of the `M` blocks is responsible for distributing the reward from one
partition of the rewards from the `EpochRewards` sysvar account.

Before each reward distribution, the `EpochRewards` account's `balance` is
checked to make sure there is enough balance to distribute the reward. After a
reward is distributed, the balance in `EpochRewards` account is decreased by the
amount of the reward that was distributed.

### Restrict Stake Account Access

To avoid the complexity and potential rewards in unexpected stake accounts,
the stake program is disabled during the epoch reward distribution period.

Any transaction that invokes staking program during this period will fail with
a new unique error code - `StakeProgramUnavailable`.

That means all updates to stake accounts have to wait until the rewards
distribution finishes.

Because different stake accounts are receiving the rewards at different blocks,
on-chain programs, which depends on the rewards of stakes accounts during the
reward period, may get into partial epoch reward state. To prevent this from
happening, loading stake accounts from on-chain programs during reward period
will be disabled. However, reading the stake account through RPC will still be
available.


## Impact

There are the two main impacts of the changes to stake accounts during the
epoch rewarding phase.

The first impact is that stake accounts will see their rewards being credited a
few blocks later in the epoch than before.

The second impact is that users will not be able to update their stakes during
the epoch reward phases, and will have to wait until the end of the epoch
reward period to make any changes.

Nonetheless, the overall amount of time that the user must wait before
receiving and updating their stake rewards should be roughly equivalent to what
they are now experiencing on the current mainnet beta, since the prolonged
first block time at the epoch boundary effectively obstructs the user's access
to those stake accounts during that time.

Another advantage with the new approach is that all non-staking transactions
will continue to be processed, while those transactions are blocked on mainnet
beta today.


## Security Considerations

While the proposed new approach does impact and modify various components of
the validators, it does not alter the economics of the reward system.

If all the changes are implemented correctly and tested fully, there are no
security issues.


## Backwards Compatibility

This is a breaking change.  The new epoch calculation and distribution approach
will not be compatible with the old approach.

## Open Questions
