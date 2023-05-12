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

* "optimistically confirmed block": a block which gets the votes of validators
with > 2/3 stake. Our algorithm tries to guarantee that an optimistically
confirmed will never be rolled back. When we are performing cluster restart, we
normally start from the highest optimistically confirmed block, but it's also
okay to start from a child of the highest optimistcially confirmed block as
long as consensus can be reached.

* `RESTART_STAKE_THRESHOLD`: We need enough validators to participate in a
restart so they can make decision for the whole cluster. If everything works
perfect, we only need 2/3 of the total stake. However, validators could die
or perform abnormally, so we currently set the `RESTART_STAKE_THRESHOLD` at
80%, which is the same as now.

## Motivation

Currently during a cluster restart, validator operators need to decide latest
optimistically confirmed slot, then restart the validators with new commandline
arguments.

The current process involves a lot of human intervention, if people make a
mistake in deciding the highest optimistically confirmed slot, it is
detrimental to the viability of the ecosystem.

We aim to automate the finding of highest optimistically confirmed slot and
block data distribution, so that we can lower the possibility of human mistakes
in the cluster restart process. This also reduces the burden on validator
operators, because they don't have to stay around while the validators
automatically try to reach consensus, they will be paged if things go wrong.

## Alternatives Considered

### Automatically detect outage and perform cluster restart
The reaction time of a human in case of emergency is measured in minutes,
while a cluster restart where human need to initiate restarts takes hours.
We consdiered various approaches to automatcially detect outage and perform
cluster restart, which can reduce recovery speed to minutes or even seconds.

However, automaticlly restarting the whole cluster still seems risky. Because
if the recovery process itself doesn't work, it might be some time before
we can get human's attention. And it doesn't solve the cases where new binary
is needed. So for now we still plan to have human in the loop.

After we gain more experience with the restart apprach in this proposal, we
may slowly try to automate more parts to improve cluster reliability.

### Use gossip and consensus to figure out restart slot before the restart
The main difference between current proposal and this proposal is that this
proposal will automatically enters restart preparation phase where local
status is exchanged via gossip without human intervention.

While this improves recovery speed, there are concerns about recovery gossip
messages interfers with normal gossip messages, and automatically start a new
message in gossip seems risky.

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
this proposal.

## Detailed Design

The new protocol tries to make all restarting validators get the same
data blocks and the same set of last votes among them, then they will almost
certainly make the same decision and proceed.

The steps roughly look like this:

1. Everyone freezes, no new blocks, no new votes, and no Turbine

2. Make all blocks which can potentially have been optimistically confirmed
before the freeze propagate to everyone

3. Make restart participants' last votes before the freeze propagate to
everyone

4. Now see if enough people can agree on one block (same slot and hash) to
restart from

4.1 If yes, proceed and restart

4.2 If no, freeze and print out what you think is wrong, wait for human

A new command line arg will be added. When the cluster is in need
of a restart, we assume validators holding at least `RESTART_STAKE_THRESHOLD`
percentage of stakes will restart with this arg. Any validators
restarted with this arg does not participate in the normal Turbine protocol,
update its vote, or generate new blocks until all of the following steps are
completed.

### Gossip last vote before the restart and ancestors on that fork

Send gossip message LastVotedForkSlots to everyone in restart, it contains the
last voted slot on its tower and the ancestor slots on the last voted fork and
is sent in a compressed bitmap like the EpicSlots data structure. The number of
ancestor slots sent is hard coded at 81000, because that's 400ms * 81000 = 9
hours, we assume most restart decisions to be made in 9 hours. If a validator
restarts after 9 hours past the outage, it cannot join the restart this way. If
enough validators failed to restart within 9 hours, then use the old restart
method.

The fields of LastVotedForkSlots are:

- `last_voted_slot`: the slot last voted, this also serves as last_slot for the
bit vector.
- `last_voted_hash`: the bank hash of the slot last voted slot.
- `slots`: compressed bit vector representing the slots on the last voted fork,
last slot is always last_voted_slot, first slot is last_voted_slot-81000.

When a validator enters restart, it increments its current shred_version, so
the gossip messages used in restart will not interfere with those outside the
restart. There is slight chance that (current_shred_version+1) % 0xffff would
collide with the new shred_version calculated after the restart, but even if
this rare case occured, we plan to flush the CRDS table on successful restart,
so gossip messages used in restart will be removed.

### Aggregate, repair, and replay the slots in LastVotedForkSlots

Aggregate the slots in received LastVotedForkSlots messages, whenever some slot
has enough stake to be optimistically confirmed and it's missing locally, start
the repair process for this slot.

We calculate "enough" stake as follows. Assume `RESTART_STAKE_THRESHOLD` is
80%. When there are 80% validators joining the restart, assuming 5% restarted 
validators can make mistakes in voting, any block with more than
67% - 5% - (100-80)% = 42% could potentially be optimistically confirmed before
the restart. If there are 85% validators in the restart, then any block with
more than 67% - 5% - (100-85)% = 47% could be optimistically confirmed before
the restart.

### Gossip current heaviest fork

After receiving LastVotedForkSlots from the validators holding stake more than 
`RESTART_STAKE_THRESHOLD` and repairing slots with "enough" stake, replay all
blocks and pick the heaviest fork as follows:

1. Pick block and update root for all blocks with more than 67% votes

2. If a picked block has more than one children, check if the votes on the
heaviest child is over threshold:

2.1 If vote_on_child + stake_on_validators_not_in_restart >= 62%, pick child.
For example, if 80% validators are in restart, child has 42% votes, then
42 + (100-80) = 62%, pick child. 62% is chosen instead of 67% because 5%
could make the wrong votes.

2.2 Otherwise stop traversing the tree and use last picked block.

After deciding heaviest block, gossip
Heaviest(X, Hash(X), received_heaviest_stake) out, where X is the latest picked
block. We also send out stake of received Heaviest messages so that we can
proceed to next step when enough validators are ready.

The fields of the Heaviest message is:

- `slot`: slot of the picked block.
- `hash`: bank hash of the picked block.
- `received`: total of stakes of the validators it received Heaviest messages
from.

### Proceed to restart if everything looks okay, halt otherwise

If things go well, all of the validators in restart should find the same
heaviest fork. But we are only sending slots instead of bank hashes in
LastVotedForkSlots, so it's possible that a duplicate block can make the
cluster unable to reach consensus. If at least 2/3 of the people agree on one
slot, they should proceed to restart from this slot. Otherwise validators
should halt and send alerts for human attention.

We will also perform some safety checks, if the voted slot does not satisfy
safety checks, then the validators will panic and halt:

- The voted slot is equal or a child of local optimistically confirmed slot.

We require that at least 80% of the people received the Heaviest messages from
validators with at least 80% stake, and that the Heaviest messages all agree on
one block and hash.

So after a validator sees that 80% of the validators received 80% of the votes,
wait for 10 more minutes so that the message it sent out have propagated, then
do the following:

- Generate a snapshot at the highest oc slot.
- Issue a hard fork at the highest oc slot and change shred version in gossip.
- Execute the current --wait-for-supermajority logic and wait for 80%.

Before a validator enters restart, it will still propagate LastVotedForkSlots
and Heaviest messages in gossip. After the restart,its shred_version will be
updated so it will no longer send or propagate gossip messages for restart.

## Impact

This proposal adds a new RepairAndRestart mode to validators, during this phase
the validators will not participate in normal cluster activities, which is the
same as now. Compared to today's cluster restart, the new mode may mean more
network bandwidth and memory on the restarting validators, but it guarantees
the safety of optimistically confirmed user transactions, and validator admins
don't need to manually generate and download snapshots again. 

## Security Considerations

The two added gossip messages LastVotedForkSlots and Heaviest will only be sent
and processed when the validator is restarted in RepairAndRestart mode. So
random validator restarting in the new mode will not bring extra burden to the
system.

Non-conforming validators could send out wrong LastVotedForkSlots and Heaviest
messages to mess with cluster restarts, these should be included in the
Slashing rules in the future.

## Backwards Compatibility

This change is backward compatible with previous versions, because validators
only enter the new mode during new restart mode which is controlled by a
command line argument. All current restart arguments like
--wait-for-supermajority and --expected-bank-hash will be kept as is for now.
