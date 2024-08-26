---
simd: '0165'
title: Async Vote Execution
authors:
  - Wen Xu
category: Standard
type: Core
status: Draft
created: 2024-08-11
feature: null
supersedes: null
superseded-by: null
extends: null
---

## Summary

Separate the execution of vote and non-vote transactions in each block. The
vote transactions and non-vote transactions will be verified and executed
independently. Because the vote transactions are normally much faster to
execute, under most cases this means vote transactions will finish
execution first.

The eventual goal is to completely separate vote and non-vote transaction
executions. But right now we still have some dependency that non-vote
transactions need to read the vote state in its parent block, so currently we
restrict that non-vote transactions cannot start execution until the vote
transactions of its parent block has finished.

## Motivation

Currently the vote transactions and non-vote transactions are mixed together in
a block, the vote transactions are only processed in consensus when the whole
block has been frozen and all transactions in the block have been verified and
executed. This is a problem because slow running non-vote transactions may affect
how fast the votes are processed and then affect the ability of consensus to
pick the correct fork.

With different hardware and running environment, there will always be some
difference on speed of transaction execution between validators. Generally
speaking, because vote transactions are so simple, the variation between vote
execution should be smaller than that between non-vote executions. Therefore,
if we only execute vote transactions in a block before voting on the block,
it is more likely validators can reach consensus faster.

Even with async vote execution, forks can still happen because of
various other situations, like network partitions or mis-configured validators.
This work just reduces the chances of forks caused by variance in non-vote
transaction executions.

The non-vote transactions do need to be executed eventually. Even though it's
hard to make sure everyone executes every block within 400ms, on average majority
of the cluster should be able to keep up.

## Alternatives Considered

To be filled.

## New Terminology

- `VED`: Vote Execution Domain, Vote transactions and all its dependencies (e.g.
fee payers for votes).
- `VED Bankhash`: The hash calculated after executing only vote transactions in
a block. If there are no votes, use default hash.
- `UED`: User Execution Domain, currently everything other than votes. We may
have more domains in the future.
- `UED Bankhash`: The hash calculated after executing only non-vote transactions
in a block. If there are no non-vote transactions, use default hash.

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

### Separating vote transactions and dependencies into a different domain

To make sure vote transactions can be executed independently, we need to
isolate its dependencies.

#### Remove clock program's dependency on votes (postponed)

The eventual goal is to introduce new transaction `ClockBump` to remove
current clock program's dependency on votes.

The transaction `ClockBump` is sent by a leader with at least 0.5% stake
every 12 slots to correct the clock drift. A small script can be used to
refund well-behaving leaders the cost of the transactions.

But currently we will keep the clock calculation as is, since the clock
sysvar uses the vote-state of the parent block to calculate average
timestamp, and we have restrictions that the UED transactions in a
block cannot start execution until the VED transactions in its
ancestors have finished. So we can currently leave clock sysvar
calculation as is.

#### Split vote accounts into two accounts in VED and UED respectively

We need to allow users move money in and out of the vote accounts, but
we also need the vote accounts to vote in VED. So there will be two accounts:

- `VoteTowerAccount`: tracks tower state and vote authority, it will be
in `VED`, it is updated by vote transactions and tracks vote credits.
- `VoteAccount`: everything else currently in vote accounts, it will be
in `UED`, users can move funds in and out of `VoteAccount` freely.

The two accounts are synced every Epoch when the rewards are calculated.

### Separate the VED and UED Domains

- Only Vote or System program can read and write accounts in `VED`
- Other programs can only read accounts in `VED`
- Users can't directly access accounts in `VED` but they can move accounts
from `VED` to `UED` and vice versa. Moving accounts from one domain to
another takes 1 Epoch, and the migration happens at Epoch boundary

### Enable Async Vote Executions

1. The leader will no longer execute any transactions before broadcasting
the block it packed. We do have block-id (Merkle tree root) to ensure
everyone receives the same block.
2. Upon receiving a new block, the validator computes the `VED bankhash`,
then vote on this block and also gives its latest `UED bankhash` on the
same fork. The `UED bankhash` will most likely be hash of an ancestor of
the received block.
3. A block is not considered Optimistically Confirmed or Finalized until
some percentage of the validators agree on the `UED bankhash`.
4. Add assertion that confirmed `UED bankhash` is not too far away from the
confirmed `VED bankhash` (currently proposed at 1/2 of the Epoch)
5. Add alerts if `UED bankhash` differs when the `VED bankhash` is the same.
This is potentially an event worthy of cluster restart.

## Impact

Since we will eliminate the impact of non-vote transaction execution speed,
we should expect to see fewer forking and late blocks.

## Security Considerations

We do need to monitor and address the possibility of UED bankhash mismatches
when VED bankhash matches.

## Backward Compatibility

Most of the changes would require feature gates.
