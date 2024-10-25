---
simd: '0165'
title: Async Vote Execution
authors:
  - Wen Xu
category: Standard
type: Core
status: Idea
created: 2024-08-11
feature: null
supersedes: null
superseded-by: null
extends: null
---

## Summary

Optimistically execute all vote transactions in a block to determine fork
selection in consensus early on, before all the transactions in the block
are fully executed and the actual fee payers for vote transactions are
checked.

This allows us to more quickly converge on one chain of blocks, so that
validators don't have to execute any blocks not on selected fork. This saves
CPU and memory resource needed in replay, it also ensures that the cluster
will have fewer forks that are caused by slow transaction execution.

## Motivation

Currently the vote transactions and non-vote transactions are mixed together in
a block, the vote transactions are only processed in consensus when the whole
block has been frozen and all transactions in the block have been verified and
executed. This is a problem because slow running non-vote transactions may
affect how fast the votes are processed and then affect the ability of
consensus to pick the correct fork. It may also mean that the leader will more
often build on a minority fork so the blocks it packed will be discarded later.

With different hardware and running environment, there will always be some
difference on speed of transaction execution between validators. Generally
speaking, because vote transactions are so simple, the variation between vote
execution should be smaller than that between non-vote executions. Also the
vote transactions are very simple and lock-free, so they normally execute
faster than non-vote transactions. Therefore, if we only execute vote
transactions in a block before voting on the block, it is more likely
validators can reach consensus faster.

Even with async vote execution, forks can still happen because of
various other situations, like network partitions or mis-configured validators.
This work just reduces the chances of forks caused by variance in non-vote
transaction executions.

The non-vote transactions do need to be executed eventually. Even though it's
hard to make sure everyone executes every block within 400ms, on average
majority of the cluster should be able to keep up.

## Alternatives Considered

### Separating vote and non-vote transactions into different domains

An earlier proposal of Async Execution proposes that we separate vote and
non-vote transactions into different domains, so that we can execute them
independently. The main concerns were:

* We need to introduce one bit in AccountsDB for every account, this
complicates the implementation

* Topping off the vote fee payer accounts becomes difficult. We need to add a
bounce account to move fees from user domain to vote domain, and the process
may take one epoch

## New Terminology

* `Ephemeral Bankhash`: The hash calculated after executing only vote
transactions in a block without checking fee payers. If there are no votes,
use default hash.
* `Final Bankhash`: The bankhash as we know it today. This is a hash calculated
after executing all transactions in a block, checking fee payers for all.

## Detailed Design

### Allow leader to skip execution of transactions (Bankless Leader)

There is already on-going effort to totally skip execution of all transactions
when leader pack new blocks. See SIMD 82, SIMD 83, and related trackers:
https://github.com/anza-xyz/agave/issues/2502

Theoretically we could reap some benefit without Bankless Leader, the leader
pack as normal, while other validators only replay votes first, then later
execute other transactions and compare with the bankhash of the leader. But in
such a setup we gain smaller speedup without much benefits, it is a possible
route during rollouts though.

### Calculate ephemeral hash executing votes only and vote on selected forks

Two new fields will be added to `TowerSync` vote transaction:

* `ephemeral_hash`: This is the hash obtained by hashing blockid and all
vote transactions in the block together. Used primarily in consensus.
* `ephemeral_slot`: This is the slot where the ephemeral_hash is calculated.

This step is optimistic in the sense that validators do not check the fee
payers when executing the vote transactions in a block. They assume vote
transactions will not fail due to insufficient fees, apply the execution
results to select the correct fork, then immediately vote on the bank with
only the hash result of the vote transactions.

This is safe because the list of validators and their corresponding stake
has already been determined at the beginning of the Epoch. The stake we used
is correct in fork selection.

To make sure the vote casted would be the same as that after replaying the
whole block, we need to be consistent on whether we mark the block dead, so
that the ephemeral hash vote doesn't vote on a block which will be marked
dead later. Currently a block can be dead for the following reasons:

1. Unable to load data from blockstore
2. Invalid block (wrong number of ticks, duplicate block, bad last fec, etc)
3. Error while set root
4. Invalid transaction

For the first two, the same check can be performed computing ephemeral hash.
We will set root on a bank only when it has full hash computed later, so the
behavior will be the same as now.

The only operation we can't check is invalid transaction, since we will skip
all non-vote transaction execution, there is no way we can check for validity
of those. The intention of this check was to prevent spams. We will remove
this check and rely on economic incentives so that the leader can perform
appropriate checks.

### Replay the full block on selected forks later

There is no protocol enforced order of block replay for various validator
implemenations, but when vote instructions are sent out, the `(slot, hash)`
is the hash of latest replayed block on the selected fork. New vote
transactions should be sent when the ephemeral hash or final hash changes.

Once a validator determined the fork it will vote on, it can prioritize
replaying blocks on the selected fork. The replay process is the same as today,
all transactions (vote and non-vote) will be executed to determine the final
bankhash. The computed bankhash will be attached to vote instructions. So we
can still detect non-determinism (same of set of instructions leading to
different results) as today, just we might find discrepancies at a later time.

In this step the validators will check the fee payers of vote transactions. So
each vote transaction is executed twice: once in the optimistical voting stage
*without* checking fee payer, and once in this stage *with* checking fee payer.
If a staked validator does not have vote fee covered for specific votes, we
will not accept the vote today, while in the future we accept the vote in fork
selection, but does not actually give vote credits because the transaction
failed.

A side benefit of this design is that the sysvars have all been calculated and
determined in the optimistical voting stage, also the fork of blocks are known,
so it may be possible to release more parallelism when executing blocks in this
stage.

### Enable Async Vote Executions

1. The leader will no longer execute any transactions before broadcasting
the block it packed. We do have block-id (Merkle tree root) to ensure
everyone receives the same block.
2. Upon receiving a new block, the validator executes only the vote
transactions without checking the fee payers. The result is immediately
applied in consensus to select a fork. Then votes are sent out for the
selected fork with the `Ephemeral bankhash` for the tip of the fork and the
most recent `Final bankhash`.
3. The blocks on the selected forks are scheduled to be replayed. When
a block is replayed, all transactions are executed with fee payers checked.
This is the same as the replay we use today.
4. A block is not considered Optimistically Confirmed or Finalized until
some percentage of the validators agree on the `Ephemeral bankhash`.
5. Add assertion that confirmed `Final bankhash` is not too far away from the
confirmed `Ephemeral bankhash` (currently proposed at 1/2 of the Epoch)
6. Add alerts if `Final bankhash` differs when the `Ephemeral bankhash` is the
same. This is potentially an event worthy of cluster restart. If more than
1/3 of the validators claim a different `Final bankhash`, halt and exit.

## Impact

Since we will eliminate the impact of non-vote transaction execution speed,
we should expect to see fewer forking and late blocks.

## Security Considerations

We do need to monitor and address the possibility of bankhash mismatches
when the tip of the fork is far away from the slot where bankhash mismatch
happened.

## Backward Compatibility

Most of the changes would require feature gates.
