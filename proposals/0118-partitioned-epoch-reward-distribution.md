---
simd: '0118'
title: Partitioned Epoch Rewards Distribution
authors: Haoran Yi (Anza), Justin Starry (Anza), Tyera Eulberg (Anza)
category: Standard
type: Core
status: Accepted
created: 2024-02-16
feature: [9bn2vTJUsUcnpiZWbu2woSKtTGW3ErZC9ERv88SDqQjK](https://github.com/anza-xyz/agave/issues/426)
supersedes: '0015'
developement:
   - Anza - [Implemented](https://github.com/anza-xyz/agave/pull/427)
   - Firedancer - Implemented
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

The major bottleneck for epoch reward distribution is to write back updates to
stake accounts. At the time of writing, there are approximately 550K active
stake accounts and 1.5K vote accounts on Solana Mainnet Beta. Given the
relatively small number of vote accounts, it makes sense to keep vote rewards
distribution mechanism unchanged. They can still be distributed efficiently at
the first block of the epoch boundary. This reduces the impact of rewards for
vote account and also simplifies the overall changes. Only stake rewards will be
distributed out over multiple blocks.

In the new stake rewards distribution approach, we will separate the
computation of rewards from the actual distribution of rewards at the epoch
boundary by dividing the process into two distinct phases:

   1. rewards calculation phase - during which the epoch rewards for all
      activate stake accounts are computed and distribution chunks are scheduled.

   1. rewards distribution phase - during which the calculated epoch rewards
      for the active stake accounts are distributed.

A new sysvar account, `EpochRewards`, will track and verify the reward
distribution during rewarding phases.


### Rewards Calculation

The reward calculation will be performed at the first block of the epoch, block
height `X`. (Some documentation on how rewards are calculated is available
[here](https://docs.solanalabs.com/consensus/stake-delegation-and-rewards#basics).)
Once the full rewards are calculated, the rewards will be partitioned into
distribution chunks stored in the bank, which will then be distributed during
the `reward distribution` phase.

To ensure that each block distributes a subset of the rewards in a
deterministic manner for the current epoch, while also randomizing the
distribution across different epochs, the partitioning of all rewards will be
done as follows.

To minimize the impact on block processing time during the reward distribution
phase, a target of 4,096 stake rewards will be distributed per block. The total
number of blocks `M` needed to distributed rewards is given by the following
formula to round up to the nearest integer without using floating point value
arithmetic:

```
M = ((4096 - 1)+num_stake_accounts)/4096
```

To safeguard against the number of stake accounts growing dramatically and
overflowing the number of blocks in an epoch, the number of blocks is capped at
10% of the total slots in an epoch. If the block cap is reached, accounts per
partition is allowed to surpass the 4,096 target.

The [SipHash 1-3](https://www.aumasson.jp/siphash/siphash.pdf) pseudo-random
function is used to hash stake account addresses efficiently and uniformly
across the blocks in the reward distribution phase. The hashing function for an
epoch is created by seeding a new `SipHasher` with the parent block's blockhash.
This hashing function can then be used to hash each active stake account's
address into a u64 hash value. The reward distribution block index `I` can then
be computed by applying the following formula to the hash:

```
I = (M * stake_address_hash) / 2^64
```

### Reward Distribution

The reward distribution phase happens after the reward computation phase, ie.
during the second block in the epoch as per the current design. It lasts for `M`
blocks (see previous section). Rewards distribution for each block occurs before
transaction processing occurs.

#### `EpochRewards` Sysvar Account

The `EpochRewards` sysvar account records whether the rewards distribution phase
is in progress, as well as the details needed to resume distribution when
starting from a snapshot during the reward distribution phase. These details
include: the parameters needed to recalculate the reward partitions, the total
rewards to be distributed, and the amount of rewards distributed so far.

The layout of `EpochRewards` sysvar is shown in the following pseudo code.
Prefixes indicate the offset of each field, and padding bytes are always zero.

```
#[repr(C, align(16))] // C representation, 16-byte alignment
struct EpochRewards{
   // little-endian unsigned 64-bit integer
   /* 0x00 */ distribution_starting_block_height: u64,
   
   // little-endian unsigned 64-bit integer
   /* 0x08 */ num_partitions: u64,
   
   // byte-array of length 32
   /* 0x10 */ parent_blockhash: Hash,

   // total points calculated for the current epoch, where points equals the sum
   // of delegated stake * credits observed for all delegations
   // little-endian unsigned 128-bit integer
   /* 0x30 */ total_points: u128, 

   // total rewards for the current epoch, in lamports
   // little-endian unsigned 64-bit integer
   /* 0x40 */ total_rewards: u64,

   // distributed rewards for the current epoch, in lamports
   // little-endian unsigned 64-bit integer
   /* 0x48 */ distributed_rewards: u64, 

   // whether the rewards period (calculation and distribution) is active
   // byte. false: 0x00, true: 0x01
   /* 0x50 */ active: bool,
}
```

The `EpochRewards` sysvar is repopulated at the start of the first block of the
epoch (before any transactions are processed), as both the total epoch rewards
and vote account rewards become available at this time. The
`distributed_rewards` field is updated per reward distribution for each block in
the reward distribution phase (before transaction processing). The `active`
field is set to false after the last partition is distributed (and
`total_rewards == distributed_rewards`), ie. before processing transactions in
block at height `X + M`.

#### Booting from Snapshot

When booting from a snapshot, a node must check the EpochRewards sysvar account
to determine whether the distribution phase is active. If so, the node must
rerun the rewards partitioning using the `EpochRewards::num_partitions` and
`EpochRewards::parent_blockhash` sysvar fields and determining the upcoming
partitions by comparing its current block height to
`EpochRewards::distribution_starting_block_height`. Then the runtime must
recalculate the remaining rewards using the `EpochRewards::total_points` and
`EpochRewards::total_rewards` sysvar fields, as well as the `EpochStakes` in the
snapshot. The recalculated rewards can be confirmed by comparing a sum of the
rewards remaining (those partitions expected to not yet have been distributed)
with the difference between the `EpochRewards::total_rewards` and
`EpochRewards::distributed_rewards` fields. Partitions for blocks prior to the
current block height can be discarded.

### Restrict Stake Account Mutation

In order to ensure partition recalculations will be the same across the
distribution period, the number and addresses of stake accounts, as well as
stake-delegation amounts, must remain unchanged. There are existing use-cases
for crediting stake accounts at epoch boundaries; so as to not disrupt these
credits, but otherwise take the most conservative approach, account credits will
be the only stake-account changes permitted during distribution.

To limit permissible actions, the Stake Program needs access to a syscall which
reports whether the distribution phase is active. This new syscall
`sol_get_epoch_rewards_sysvar` should return the values of the `EpochRewards`
sysvar. All Stake Program instructions that mutate stake data or debit stake
account `lamports` balances must be disabled when `EpochRewards::active` is true.

Any transaction that attempts to invoke such an instruction will fail with this
new error code:

```
StakeError {
   EpochRewardsActive = 16,
}
```

Other users can access the `active` field from the
`sol_get_epoch_rewards_sysvar` syscall to determine the distribution-phase
status from within the SVM.

## Impact

There are the two main impacts of the changes to stake accounts during the
epoch rewarding phase.

The first impact is that stake accounts will see their rewards being credited a
few blocks later in the epoch than before.

The second impact is that users will only be able to credit their stake accounts
during the epoch reward phase. Any other updates will have to wait until the end
of the phase.

Nonetheless, the overall amount of wall time that the user must wait before
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

Reward distribution relies on restricting any lamport debit or state changes for
stake accounts until distribution is completed.

The initial reward calculation and reward-distribution progress should be
recoverable from snapshots produced during the reward distribution period to
avoid consensus failure.


## Backwards Compatibility

This is a breaking change.  The new epoch calculation and distribution approach
will not be compatible with the old approach.

## Open Questions
