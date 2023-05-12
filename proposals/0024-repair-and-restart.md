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

Improve the current [cluster restart](https://docs.solana.com/running-validator/restart-cluster)
procedure such that validators can automatically figure out the highest
optimistically confirmed slot and then proceed to restart if everything looks
fine.

## New Terminology

None

## Motivation

Currently during a cluster restart, validator operators need to decide latest
optimistically confirmed slot, then restart the validators with new commandline
arguments.

The current process involves a lot of human intervention, if people make a
mistake in deciding the highest optimistically confirmed slot, it could mean
rollback of user transactions after those transactions have been confirmed,
which is not acceptable.

We aim to automate the negotiation of highest optimistically confirmed slot and
block data distribution, so that we can lower the possibility of human mistakes
in the cluster restart process.

## Alternatives Considered

See [Handling Common Solana Outages](https://docs.google.com/document/d/1RkNAyz-5aKvv5FF44b8SoKifChKB705y5SdcEoqMPIc)
for details.

There are many proposals about automatically detecting that the cluster is
in an outage so validators should enter a recovery process automatically.

While getting human out of the loop greatly improves recovery speed,
automaticlly restarting the whole cluster still seems risky. Because if
the recovery process itself doesn't work, it might be some time before
we can get human's attention. And it doesn't solve the cases where new binary
is needed. So for now we still plan to have human in the loop.

## Detailed Design

The new protocol tries to make all 80% restarting validators get the same
data blocks and the same set of last votes among them, then they can probably
make the same decision and then proceed.

The steps roughly look like this:

1. Everyone freezes, no new blocks, no new votes, and no Turbine

2. Make all blocks which can potentially have been optimistically confirmed
before the freeze propagate to everyone

3. Make restart participants' last votes before the freeze propagate to
everyone

4. Now see if enough people can optimistically agree on one block (same slot
and hash) to restart from

4.1 If yes, proceed and restart

4.2 If no, freeze and print out what you think is wrong, wait for human

A new command line arg --RepairAndRestart is added. When the cluster is in need
of a restart, we assume at least 80% will restart with this arg. Any validators
restarted with this arg does not participate in the normal Turbine protocol,
update its vote, or generate new blocks until all of the following steps are
completed.

### Gossip last vote before the restart and ancestors on that fork

Send Gossip message LastVotedForkSlots to everyone in restart, it contains the
last voted slot on its tower and the ancestor slots on the last voted fork and
is sent in a compressed bitmap like the `EpochSlots` data structure. The number of
ancestor slots sent is hard coded at 81000, because that's 400ms * 81000 = 9
hours, we assume most restart decisions to be made in 9 hours. If a validator
restarts after 9 hours past the outage, it cannot join the restart this way. If
enough validators failed to restart within 9 hours, then fallback to the
manual, interactive cluster restart method.

The fields of LastVotedForkSlots are:

- `last_voted_slot`: the slot last voted, this also serves as last_slot for the
bit vector.
- `last_voted_hash`: the bank hash of the slot last voted slot.
- `slots`: compressed bit vector representing the slots on the last voted fork,
last slot is always last_voted_slot, first slot is last_voted_slot-81000.

When a validator enters restart, it increments its current shred_version, so
the Gossip messages used in restart will not interfere with those outside the
restart. There is slight chance that (current_shred_version+1) % 0xffff would
collide with the new shred_version calculated after the restart, but even if
this rare case occured, we plan to flush the CRDS table on successful restart,
so Gossip messages used in restart will be removed.

### Aggregate, repair, and replay the slots in LastVotedForkSlots

Aggregate the slots in received LastVotedForkSlots messages, whenever some slot
has enough stake to be optimistically confirmed and it's missing locally, start
the repair process for this slot.

We calculate "enough" stake as follows. When there are 80% validators joining
the restart, assuming 5% restarted validators can make mistakes in voting, any
block with more than 67% - 5% - (100-80)% = 42% could potentially be
optimistically confirmed before the restart. If there are 85% validators in the
restart, then any block with more than 67% - 5% - (100-85)% = 47% could be
optimistically confirmed before the restart.

### Gossip current heaviest fork

After receiving LastVotedForkSlots from 80% of the validators and reparing
slots with "enough" stake, replay all blocks and pick the heaviest fork as
follows:

1. Pick block and update root for all blocks with more than 67% votes

2. If a picked block has more than one children, check if the votes on the
heaviest child is over threshold:

2.1 If vote_on_child + stake_on_validators_not_in_restart >= 62%, pick child.
For example, if 80% validators are in restart, child has 42% votes, then
42 + (100-80) = 62%, pick child. 62% is chosen instead of 67% because 5%
could make the wrong votes.

2.2 Otherwise stop traversing the tree and use last picked block.

After deciding heaviest block, Gossip
Heaviest(X, Hash(X), received_heaviest_stake) out, where X is the latest picked
block. We also send out stake of received Heaviest messages so that we can
proceed to next step when enough validators are ready.

The fields of the Heaviest message is:

- `slot`: slot of the picked block.
- `hash`: bank hash of the picked block.
- `received`: total of stakes of the validators it received Heaviest messages
from.

### Proceed to restart if everything looks okay, halt otherwise

If things go well, all 80% of the validators should find the same heaviest
fork. But we are only sending slots instead of bank hashes in
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
- Issue a hard fork at the highest oc slot and change shred version in Gossip.
- Execute the current --wait-for-supermajority logic and wait for 80%.

Before a validator enters restart, it will still propagate LastVotedForkSlots
and Heaviest messages in Gossip. After the restart,its shred_version will be
updated so it will no longer send or propagate Gossip messages for restart.

## Impact

This proposal adds a new RepairAndRestart mode to validators, during this phase
the validators will not participate in normal cluster activities, which is the
same as now. Compared to today's cluster restart, the new mode may mean more
network bandwidth and memory on the restarting validators, but it guarantees
the safety of optimistically confirmed user transactions, and validator admins
don't need to manually generate and download snapshots again. 

## Security Considerations

The two added Gossip messages LastVotedForkSlots and Heavist will only be sent
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
However, this change does not work until at least 80% installed the new binary
and they are willing to use the new methods for restart.
