---
simd: '0024'
title: Automatially repair and start for a cluster restart
authors:
  - Wen Xu (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2023-04-07
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Improve the current [cluster restart]
(https://docs.solana.com/running-validator/restart-cluster)
procedure such that validators can automatically figure out confirmed slots
and then proceed to restart if everything looks fine.

## Motivation

Currently during a cluster restart, validator operators need to decide latest
optimistically confirmed slot, then restart the validators with new commandline
arguments.

The current process involves a lot of human intenvention, if people make a
mistake in deciding the highest optimistically confirmed slot, it could mean
rollback of user transactions after they have been confirmed, which is not
acceptable.

We aim to automate the finding of highest optimistically confirmed slot and
block data distribution, so that we can lower the possibility of human mistakes
in the cluster restart process.

## Alternatives Considered

See [Handling Common Solana Outages]
(https://docs.google.com/document/d/1RkNAyz-5aKvv5FF44b8SoKifChKB705y5SdcEoqMPIc)
for details.

There are many proposals about automatically detecting that the cluster is
in an outage so validators should enter a recovery process automatically.

While getting human out of the loop greatly improves recovery speed,
automaticlly restarting the whole cluster still seems risky. Because if
the recovery process itself doesn't work, it might be some time before
we can get human's attention. And it doesn't solve the cases where new binary
is needed. So for now we still plan to have human in the loop.

## New Terminology

* SilentRepairMode - when validators restart in this new mode, they will
talk with each other to find the highest optimistically confirmed slot and
repair all necessary blocks. To improve speed and guarantee simplicity,
Turbine, vote, and new block generation are paused in this mode.

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

4. Now see if enough people can optimistically agree on one block (same
slot and hash) to restart from

4.1 If yes, proceed and restart

4.2 If no, freeze and print out what you think is wrong, wait for human

A new command line arg --RepairAndRestart <optional_slots_to_send> is added.
When the cluster is in need of a restart, we assume at least 80% will restart
with this arg. Any validators restarted with this arg does not participate in
the normal Turbine protocol, update its vote, or generate new blocks until all
of the following steps are completed.

### Gossip last vote before the restart and ancestors on that fork

Send Gossip message LastVotedForkSlots to everyone, it contains the last voted
slot on its tower and the ancestor slots on the last voted fork and is sent in
a bitmap like the EpicSlots data structure. The number of ancestor slots sent
is determined by <optional_slots_to_send_on_last_voted_fork>. By default this
number is 108000, because that's 400ms * 10800 = 12 hours, we assume most
restart decisions to be made in half a day. You can increase this number if you
restart after the outage lasted more than 12 hours.

### Aggregate, repair, and replay the slots in LastVotedForkSlots

Aggregate the slots in received LastVotedForkSlots messages, whenever some slot
has more than 34% stake and it's missing locally, start the repair process for
this slot. 34% is chosen because we don't want to miss any optimistically
confirmed slot, those slots have at least 67% votes, there can be 33% validators
not giving out dependable answers (for example, claim they didn't vote for a slot
when they actually did), so we need to repair all slots with at least
67% - 33% = 34% votes.

### Gossip current heaviest fork

After receiving LastVotedForkSlots from 80% of the validators and reparing all
slots with more than 34% votes, count the heaviest fork and Gossip
Heaviest(X, Hash(X)) out, where X is the tip of the heaviest fork.

### Proceed to restart if everything looks okay, halt otherwise

If things go well, all 80% of the validators should find the same heaviest fork. But
we are only sending slots instead of bank hashes in LastVotedForkSlots, so it's
possible that a duplicate block can make the cluster unable to reach consensus. If
at least 2/3 of the people agree on one slot, they should proceed to restart from
this slot. Otherwise validators should halt and send alerts for human attention.

## Impact

This proposal adds a new RepairAndRestart mode to validators, during this phase
the validators will not participate in normal cluster activities, which is the
same as now. Compared to today's cluster restart, the new mode may mean more
network bandwidth and memory on the restarting validators, but it guarantees the
safety of optimistically confirmed user transactions, and validator admins don't
need to manually generate and download snapshots again. 

## Security Considerations

The two added Gossip messages LastVotedForkSlots and Heavist will only be sent and
processed when the validator is restarted in RepairAndRestart mode. So random validator
restarting in the new mode will not bring extra burden to the system.

Non-conforming validators could send out wrong LastVotedForkSlots and Heaviest
messages to mess with cluster restarts, these should be included in the Slashing
rules in the future.

## Backwards Compatibility

This change is backward compatible with previous versions, because validators only
enter the new mode during new restart mode which is controlled by a command line
argument. All current restart arguments like  --wait-for-supermajority and
--expected-bank-hash will be kept as is for now.
However, this change does not work until at least 80% installed the new binary and
they are willing to use the new methods for restart.
