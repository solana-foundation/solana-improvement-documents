---
simd: '0015'
title: Partitioned Epoch Reward Distribution
authors:
  - Haoran Yi (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2023-03-02
feature: (fill in with feature tracking issues once accepted)
---

## Summary

A new way to distribute epoch rewards across multiple blocks is proposed to
address the current performance problems associated with the reward
distribution in a single block at the start of the epoch.

## Motivation

The distribution of epoch rewards at the start block of an epoch becomes a
significant bottleneck due to the rising number of stake accounts and voting
nodes on the network.

To address this bottleneck, we propose a new approach for distributing the
epoch rewards over multiple blocks.

## New Terminology

   - rewards calculation: calculate the epoch rewards for all activate stake
     accounts

   - rewards distribution: distribute the epoch rewards for the active stake
     accounts

## Alternatives Considered

We have discussed two alternative approaches.

The first approach is to set a threshold on stake balance, and limit the epoch
rewards to the accounts with stake balance above the threshold. This will
effectively reduce the number of stake rewards to be distributed, and reduce
reward distribution time at the epoch boundary. However, this will impact stake
accounts with small balance. To receive rewards, the small stake accounts will
be forced to join stake pools, which some may be hesitant to do.

The second approach is to handle reward distributions through transactions with
specialized native reward programs. While this approach is optimal, it requires
significant modifications and doesn't align with the current reward code.

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

### Rewards Calculation

Reward calculation phase computes all the rewards that need to be distributed
for the active stake accounts, and partitions the reward into a number of
chunks for distribution in the next phase.

To help track and verify the reward distribution in the next phase, two new
kinds of sysvar accounts are introduced: (a) `EpochRewardHistory` sysvar
account and (b) `EpochRewardReserve` sysvar accounts. These sysvar accounts are
updated at the boundary blocks (i.e. first and last blocks) in the reward
calculation phase. More details of these two kinds of sysvar accounts are
described in the following sections.

The reward computation phase lasts *exactly* for `N` block heights since
the beginning of the epoch. When reaching block height `N` after the start
block of the `reward calculation phase`, the bank will mark it the end the
`reward calculation phase`.

On Solana Mainnet Beta, the 90% cut-off of the epoch reward computation time
for all the nodes is around 40 seconds for the ~550K active stake accounts.
This is approximately 1000 blocks at the cluster's average block rate (40ms
per block, approximately 10% of the total block time, i.e. 400ms). Therefore,
a conservative value of 1,000 is chosen for `N`. This parameter will be
feature-gated and maybe updated in future.

At the end of the `reward calculation phase`, aka. `Epoch_start + N` block
height, the reward computation process is "barrier synchronized" with the bank
replay process. If the reward calculation is slow and hasn't complete the
computation at block-height `Epoch_start + N`, the bank replay process will
block and wait until the reward computation is finished, before processing any
blocks with height greater than `N`.

After the full rewards result is calculated, the result is partitioned into
distribution chunks, which will be distributed during the `reward distribution`
phase.

To ensure that each block distributes a subset of the rewards in a
deterministic manner for the current epoch, while also randomizing the
distribution across different epochs, the partitioning of all rewards will be
done as follows.

First, the reward results are sorted by account's Pubkey, and randomly shuffled
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

### `EpochRewardHistory` Sysvar Account

`EpochRewardHistory` sysvar account maintains a history of epoch rewards. The
internal format of `EpochRewardHistory` is an array of `RewardHistoryEntry`,
where each entry corresponds to the reward for one particular epoch. The
maximum depth of the history is chosen to be the same as the depth of stake
history for consistency, which is 512.

The layout of `EpochRewardHistory` sysvar is shown in the following pseudo code.

```
struct RewardHistoryEntry {
   total_reward_in_lamport: u64,          // total rewards for this epoch
   distributed_reward_in_lamport: u64,    // already distributed reward amount
   root_hash: Option<Hash>,               // hash computed from all EpochRewardReserves
}

type EpochReward = (Epoch, RewardHistoryEntry);

struct RewardHistory {
   rewards: [EpochReward; 512],
}
```

The `EpochRewardHistory` is updated at the start of the first block of the
epoch (before any transactions are processed), as both the total epoch rewards
and vote account rewards become available at this time. Once the vote account
rewards have been distributed, a `RewardHistoryEntry` will be added to the
`EpochRewardHistory` array with the total amount of rewards and the amount
distributed, which is equal to the sum of the vote account rewards. If the
length of the history array exceeds the maximum depth (512), the oldest entries
will be removed from the array (i.e., "popped").

The `root_hash` is the hash of all the `EpochRewardReserve` accounts hashes.
Initially, at the start of the epoch's first block, the `root_hash` is set to
`None`. Once all the `EpochRewardReserve` accounts are filled and the reward
computation phase is complete, the `root_hash` is calculated by hashing all the
`EpochRewardReserve` hashes, which is defined below.

```
root_hash = Hash(EpochRewardReserve[..].reserve_hash)



                     +-------------+
                     |             |
                     |  root_hash  |
                     |             |
                     +------+------+
                            |
    +--------------+--------+-----------+
    |              |                    |
+---+----+   +-----+--+             +---+-----+
|reserve |   | reserve|   ......    | reserve |
| hash   |   |  hash  |             |  hash   |
+--------+   +--------+             +---------+

```

### `EpochRewardReserve` Sysvar Account

`EpochRewardReserve` sysvar accounts track the rewards to be distributed for each
individual block in the `reward distribution phase`.

The address of `EpochRewardReserve` account for a particular block is specified
by PDA as follows

```
fn get_epoch_reward_reserve_addr(epoch: u64, distribution_phase_index: u64)
   -> (Pubkey, u8)
{
   let (address, bump_seed) = Pubkey::find_program_address(&[
        b"inflation-rewards-distributor",
        b"stake",
        &epoch.to_le_bytes(),
        &distribution_phase_index.to_le_bytes(),
    ],
    &sysvar::id(),
   );
   (address, bump_seed)
}
```

where `distribution_phase_index` is the block height offset since the start of
reward distribution phase.


```
struct EpochRewardReserve {
   reserve_hash: Hash,
   reward_balance: u64,
}
```

The account balance of `EpochRewardsReserve` is equal to the total amount (in the
`AccountInfo::lamports` field) of all rewards to be distributed in a block. And
the `reward_balance` field shadow the `AccountInfo::lamports`. The
`reserve_hash` is computed from the rewards in the block as follows.

```
reserve_hash = hash([(stake_account_key, reward_amount)])
```

At the end of reward computation phase, `M` `EpochRewardsReserve` sysvar
accounts are updated, where `M` corresponds to how many blocks will be in
the reward distribution phase, which is described in the next section.


### Reward Distribution

Reward distribution phase happens right after reward computation phase and
lasts for `M` blocks. Each of the `M` blocks is responsible for distributing
the reward from one partition of the rewards stored in the `EpochRewardReserve`
account with the address specified by the `get_epoch_reward_reserve_addr`
function above. Once all rewards have been distributed, the balance of the
`EpochRewardReserve` account and shadow balance field `reward_balance` both
should be reduced to `0`. However, to protect from unexpected deposits to
`EpochRewardReserve` accounts, when the reward distribution completes, any
extra lamports in `EpochRewardReserve` accounts will be burned.

Before each reward distribution, the `EpochRewardReserve` account's
`reserve_balance` is checked to make sure there is enough balance to distribute
the reward. After a reward is distributed, the balance and `reserve_balance` in
`EpochRewardReserve` account is decreased by the amount of the reward that was
distributed. The hash of reward account address and reward amount is
accumulated when rewards are distributed. After all the rewards are distributed
from the reserve account for the block, the hash computed from all the rewards
distribution is verified against the hash stored in the `EpochRewardReserve`
account to make sure they match.

During the reward distribution phase, there is a "root accumulator hash" that
starts empty and is gradually filled by adding each hash of a
`EpochRewardReserve`. The distribution process continues until the "root
accumulator hash" matches the "root hash" stored in `EpochRewardHistory`. At this
point, the reward distribution for the current epoch is complete and we can
safely assume that there will be no further distributions for the remainder of
the epoch.

### RPC API Changes

To accommodate the new reward distribution approach, there are a few changes
to the rewards related RPC APIs.

In particular, `getInflationReward` RPC API will change. It will be unavailable
during reward period, and only available after reward distribution completes.

To help to determine when the reward period completes, a new RPC API,
`getRewardPeriodInfo`,  will be added. This API will returns:

   - `numOfRewardComputationBlocks: u64`
   - `numOfRewardDistributionBlocks: u64`
   - `totalNumOfBlocksInRewardPeriod: u64`.

To query the reward received for individual account, user Apps will first make
a query to `getRewardPeriodInfo`, and wait for `totalNumOfBlocksInRewardPeriod`
of blocks since the start of the epoch. Then, the App can call
`getInflationReward` to query the rewards as before.


### Restrict Stake Account Access

To avoid the complexity and potential rewards in unexpected stake accounts,
the stake program is disabled during the epoch reward period.

Any transaction that invokes staking program during this period will fail with
a new unique error code - `StakeProgramUnavailable`.

That means all updates to stake accounts have to wait until the rewards
distribution finishes.


### Snapshot and Cluster Restart

Due to the fact that rewards are now distributed over multiple blocks, accounts
snapshots that are taken during reward distribution phase, need to store the
reward calculation result.

```
struct RewardCalculationResult {
   partitions: Vec<[(Pubkey, u64); 4096]>,
}
```

The bank snapshot data file inside snapshot archives will have a new optional
field, `Option<RewardCalculationResult>`, at the end. All snapshots taken during
the reward period will have this field populated.

When a node restarts from those snapshots, it will pick up the
`RewardCalculationResult` from the snapshot and resume the left-over reward
distribution.

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
