---
simd: '0024'
title: Optimistic cluster restart automation
authors:
  - Wen Xu (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2023-04-07
feature: (fill in with feature tracking issues once accepted)
---

## Summary

During a cluster restart following an outage, use gossip to exchange local
status and automatically reach consensus on the block to restart from. Proceed
to restart if validators in the restart can reach agreement, or print debug
information and halt otherwise.

## New Terminology

* "cluster restart": When there is an outage such that the whole cluster
stalls, human may need to restart most of the validators with a sane state so
that the cluster can continue to function. This is different from sporadic
single validator restart which does not impact the cluster. See
[cluster restart](https://docs.solana.com/running-validator/restart-cluster)
for details.

* "optimistically confirmed block": a block which gets the votes from the
majority of the validators in a cluster (> 2/3 stake). Our algorithm tries to
guarantee that an optimistically confirmed will never be rolled back. When we 
are performing cluster restart, we normally start from the highest 
optimistically confirmed block, but it's also okay to start from a child of the 
highest optimistically confirmed block as long as consensus can be reached.

* "silent repair phase": During the proposed optimistic cluster restart automation process, the validators in
restart will first spend some time to exchange information, repair missing
blocks, and finally reach consensus. The validators only continue normal block
production and voting after consensus is reached. We call this preparation
phase where block production and voting are paused the silent repair phase.

* "ephemeral shred version": right now we update `shred_version` during a
cluster restart, it is used to verify received shreds and filter Gossip peers. 
In the new repair and restart plan, we introduce a new temporary shred version 
in the silent repair phase so validators in restart don't interfere with those 
not in restart. Currently this ephemeral shred version is calculated using
`(current_shred_version + 1) % 0xffff`.

* `RESTART_STAKE_THRESHOLD`: We need enough validators to participate in a
restart so they can make decision for the whole cluster. If everything works
perfect, we only need 2/3 of the total stake. However, validators could die
or perform abnormally, so we currently set the `RESTART_STAKE_THRESHOLD` at
80%, which is the same as now.

## Motivation

Currently during a cluster restart, validator operators need to decide the
highest optimistically confirmed slot, then restart the validators with new
command-line arguments.

The current process involves a lot of human intervention, if people make a
mistake in deciding the highest optimistically confirmed slot, it is
detrimental to the viability of the ecosystem.

We aim to automate the negotiation of highest optimistically confirmed slot and
the distribution of all blocks on that fork, so that we can lower the 
possibility of human mistakes in the cluster restart process. This also reduces 
the burden on validator operators, because they don't have to stay around while 
the validators automatically try to reach consensus, they will be paged if 
things go wrong.

## Alternatives Considered

### Automatically detect outage and perform cluster restart
The reaction time of a human in case of emergency is measured in minutes,
while a cluster restart where human initiate validator restarts takes hours.
We considered various approaches to automatically detect outage and perform
cluster restart, which can reduce recovery speed to minutes or even seconds.

However, automatically restarting the whole cluster seems risky. Because
if the recovery process itself doesn't work, it might be some time before
we can get human's attention. And it doesn't solve the cases where new binary
is needed. So for now we still plan to have human in the loop.

After we gain more experience with the restart approach in this proposal, we
may slowly try to automate more parts to improve cluster reliability.

### Use gossip and consensus to figure out restart slot before the restart
The main difference between current proposal and this proposal is that this
proposal will automatically enter restart preparation phase without human 
intervention.

While getting human out of the loop improves recovery speed, there are concerns 
about recovery gossip messages interfering with normal gossip messages, and 
automatically start a new message in gossip seems risky.

### Automatically reduce block production in an outage
Right now we have vote-only mode, a validator will only pack vote transactions
into new blocks if the tower distance (last_vote - local_root) is greater than
400 slots.

Unfortunately in the previous outages vote-only mode isn't enough to save the
cluster. There are proposals of more aggressive block production reduction to
save the cluster. For example, a leader could produce only one block in four
consecutive slots allocated to it.

However, this only solves the problem in specific type of outage, and it seems
risky to aggressively reduce block production, so we are not proceeding with
this proposal for now.

## Detailed Design

The new protocol tries to make all restarting validators get the same data
blocks and the same set of last votes, then they will almost certainly make the 
same decision on the canonical fork and proceed.

A new command line arg will be added. When the cluster is in need
of a restart, we assume validators holding at least `RESTART_STAKE_THRESHOLD`
percentage of stakes will restart with this arg. Then the following steps
will happen:

1. The operator restarts the validator with a new command-line argument to cause it to enter the silent repair phase at boot, where it will not make new 
blocks or change its votes. The validator propagates its local voted fork
information to all other validators in restart.

2. While counting local vote information from all others in restart, the
validator repairs all blocks which could potentially have been optimistically
confirmed.

3. After repair is complete, the validator counts votes on each fork and
sends out local heaviest fork.

4. Each validator counts if enough nodes can agree on one block (same slot and
hash) to restart from:

  1. If yes, proceed and restart

  2. If no, print out what it thinks is wrong, halt and wait for human

See each step explained in details below.

### 1. Gossip last vote before the restart and ancestors on that fork

The main goal of this step is to propagate the locally selected fork to all
others in restart.

We use a new Gossip message `LastVotedForkSlots`, its fields are:

- `last_voted_slot`: `u64` the slot last voted, this also serves as last_slot
for the bit vector.
- `last_voted_hash`: `Hash` the bank hash of the slot last voted slot.
- `ancestors`: `BitVec<u8>` compressed bit vector representing the slots on
sender's last voted fork. the most significant bit is always
`last_voted_slot`, least significant bit is `last_voted_slot-81000`.

The number of ancestor slots sent is hard coded at 81000, because that's
400ms * 81000 = 9 hours, we assume most restart decisions to be made in 9 
hours. If a validator restarts after 9 hours past the outage, it cannot join 
the restart this way. If enough validators failed to restart within 9 hours, 
then fallback to the manual, interactive cluster restart method.

When a validator enters restart, it uses ephemeral shred version to avoid
interfering with those outside the restart. There is slight chance that 
the ephemeral shred version would collide with the shred version after the
silent repair phase, but even if this rare case occurred, we plan to flush the 
CRDS table on successful restart, so gossip messages used in restart will be 
removed.

### 2. Repair ledgers up to the restart slot

The main goal of this step is to repair all blocks which could potentially be
optimistically confirmed.

We need to prevent false negative at all costs, because we can't rollback an 
optimistically confirmed block. However, false positive is okay. Because when 
we select the heaviest fork in the next step, we should see all the potential 
candidates for optimistically confirmed slots, there we can count the votes and
remove some false positive cases.

However, it's also overkill to repair every block presented by others. When
`LastVotedForkSlots` messages are being received and aggregated, a validator
can categorize blocks missing locally into 3 categories: ignored, must-have, 
and unsure. Depending on the stakes of validators currently in restart, some
slots with too few stake can be safely ignored, some have enough stake they
should definitely be repaired, and the rest would be undecided pending more
confirmations.

Assume `RESTART_STAKE_THRESHOLD` is 80% and that 5% restarted validators can
make mistakes in voting.

When only 5% validators are in restart, everything is in "unsure" category.

When 67% validators are in restart, any slot with less than
67% - 5% - (100-67%) = 29% is in "ignored" category, because even if all 
validators join the restart, the slot will not get 67% stake. When this 
threshold is less than 33%, we temporarily put all blocks with >33% stake into  
"must-have" category to speed up repairing. Any slot with between 29% and 33% 
stake is "unsure".

When 80% validators are in restart, any slot with less than
67% - 5% - (100-80%) = 42% is in "ignored" category, the rest is "must-have".

From above examples, we can see the "must-have" threshold changes dynamically 
depending on how many validators are in restart. The main benefit is that a
block will only move from "must-have/unsure" to "ignored" as more validators 
join the restart, not vice versa. So the list of blocks a validator needs to
repair will never grow bigger when more validators join the restart.

### 3. Gossip current heaviest fork

The main goal of this step is to "vote" the heaviest fork to restart from.

We use a new Gossip message `HeaviestFork`, its fields are:

- `slot`: `u64` slot of the picked block.
- `hash`: `Hash` bank hash of the picked block.
- `received`: `u8` total percentage of stakes of the validators it received
`HeaviestFork` messages from.

After receiving `LastVotedForkSlots` from the validators holding stake more 
than  `RESTART_STAKE_THRESHOLD` and repairing slots in "must-have" category,
replay all blocks and pick the heaviest fork as follows:

1. For all blocks with more than 67% votes, they must be on picked fork.

2. If a picked block has more than one children, check if the votes on the
heaviest child is over threshold:

  1. If vote_on_child + stake_on_validators_not_in_restart >= 62%, pick child.
For example, if 80% validators are in restart, child has 42% votes, then
42 + (100-80) = 62%, pick child. 62% is chosen instead of 67% because 5%
could make the wrong votes.

It's okay to use 62% here because the goal is to prevent false negative rather
than false positive. If validators pick a child of optimistically confirmed
block to start from, it's okay because if 80% of the validators all choose this
block, this block will be instantly confirmed on the chain.

  2. Otherwise stop traversing the tree and use last picked block.

After deciding heaviest block, gossip
`HeaviestFork(X, Hash(X), received_heaviest_stake)` out, where X is the latest
picked block. We also send out stake of received `HeaviestFork` messages so 
that we can proceed to next step when enough validators are ready.

### 4. Proceed to restart if everything looks okay, halt otherwise

All validators in restart keep counting the number of `HeaviestFork` where
`received_heaviest_stake` is higher than 80%. Once a validator counts that 80%
of the validators send out `HeaviestFork` where `received_heaviest_stake` is 
higher than 80%, it starts the following checks:

- Whether all `HeaviestFork` have the same slot and same block Hash. Because
validators are only sending slots instead of bank hashes in 
`LastVotedForkSlots`, it's possible that a duplicate block can make the
cluster unable to reach consensus. So block hash needs to be checked as well.

- The voted slot is equal or a child of local optimistically confirmed slot.

If all checks pass, the validator immediately starts generation of snapshot at
the agreed upon slot.

While the snapshot generation is in progress, the validator also checks to see
whether two minutes has passed since agreement has been reached, to guarantee 
its `HeaviestFork` message propagates to everyone, then proceeds to restart:

1. Issue a hard fork at the designated slot and change shred version in gossip.
2. Execute the current tasks in --wait-for-supermajority and wait for 80%.

Before a validator enters restart, it will still propagate `LastVotedForkSlots`
and `HeaviestFork` messages in gossip. After the restart,its shred_version will 
be updated so it will no longer send or propagate gossip messages for restart.

If any of the checks fails, the validator immediately prints out all debug info,
sends out metrics so that people can be paged, and then halts.

## Impact

This proposal adds a new silent repair mode to validators, during this phase
the validators will not participate in normal cluster activities, which is the
same as now. Compared to today's cluster restart, the new mode may mean more
network bandwidth and memory on the restarting validators, but it guarantees
the safety of optimistically confirmed user transactions, and validator admins
don't need to manually generate and download snapshots again. 

## Security Considerations

The two added gossip messages `LastVotedForkSlots` and `HeaviestFork` will only
be sent and processed when the validator is restarted in RepairAndRestart mode.
So random validator restarting in the new mode will not bring extra burden to
the system.

Non-conforming validators could send out wrong `LastVotedForkSlots` and
`HeaviestFork` messages to mess with cluster restarts, these should be included
in the Slashing rules in the future.

## Backwards Compatibility

This change is backward compatible with previous versions, because validators
only enter the new mode during new restart mode which is controlled by a
command line argument. All current restart arguments like
--wait-for-supermajority and --expected-bank-hash will be kept as is for now.
