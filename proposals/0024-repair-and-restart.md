---
simd: '0024'
title: Automatially find confirmed slots and repair them before a cluster restart
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
procedure such that validators can automatically figure out confirmed slots,
generate a snapshot, and then proceed to restart if everything looks fine.

## Motivation
Currently during a [cluster restart](https://docs.solana.com/running-validator/restart-cluster), validator operators need to decide latest optimistically confirmed slot, create a snapshot,
then restart the validators with new commandline arguments.

The current process involves a lot of human intenvention, if people make a
mistake in deciding the highest optimistically confirmed slot, it could mean
rollback of user transactions after they have been confirmed, which is not
acceptable.

We aim to automate the finding of the highest optimistically confirmed slot and snapshot generation, so that
we can lower the possibility of human mistakes in the cluster restart process.

## Alternatives Considered

See [Handling Common Solana Outages](https://docs.google.com/document/d/1RkNAyz-5aKvv5FF44b8SoKifChKB705y5SdcEoqMPIc/edit#bookmark=id.jtrjbe6g4mk3) for details.

There are many proposals about automatically detecting that the cluster is
in an outage so validators should enter a recovery process automatically.

While getting human out of the loop greatly improves recovery speed,
automaticlly restarting the whole cluster still seems risky. Because if
the recovery process itself doesn't work, it might be some time before
we can get human's attention. And it doesn't solve the cases where new binary
is needed. So for now we still plan to have human in the loop.

## New Terminology

* SilentRepairMode - when validators restart in this new mode, they will
talk with each other to find the highest optimistically confirmed slot and repair all necessary
blocks. To improve speed and guarantee simplicity, Turbine, vote, and new
block generation are paused in this mode.

## Detailed Design

A new command line arg --RepairAndRestart <optional_slots_to_send_on_last_voted_fork> is
added. When the cluster is in need of a restart, we assume at least 80% will
restart with this arg. Any validators restarted with this arg does not participate
in the normal Turbine protocol, update its vote, or generate new blocks until all
of the following steps are complted.

### Gossip last vote before the restart and ancestors on that fork
Send Gossip message LastVotedForkSlots to everyone, it contains the last voted slot on
its tower and the ancestor slots on the last voted fork and is sent in a bitmap like
the EpicSlots data structure. The number of ancestor slots sent is determined by
<optional_slots_to_send_on_last_voted_fork>, by default this number is 2000.

### Aggregate, repair, and replay the slots in LastVotedForkSlots
Aggregate the slots in received LastVotedForkSlots messages, whenever some slot has
more than 47% stake and it's missing locally, start the repair process for this slot.
47% is chosen because we don't want to miss any optimistically confirmed slot, those
slots have at least 67% votes, and we assume that there can be 20% validators not
participating in restarts, so we need to repair all slots with at least
67% - 20% = 47% votes.

### Gossip current heaviest fork
After receiving LastVotedForkSlots from 80% of the validators and reparing all slots
with more than 47% votes, count the heaviest fork and Gossip Heaviest(X, Hash(X)) out,
where X is the tip of the heaviest fork.

### Generate local snapshot on the heaviest slot
Generate a local snapshot on the heaviest slot.

### Proceed to restart if everything looks okay, halt otherwise
If things go well, all 80% of the validators should find the same heaviest fork. But
we are only sending slots instead of bank hashes in LastVotedForkSlots, so it's possible
that a duplicate block can make the cluster unable to reach consensus. If at least
2/3 of the people agree on one slot, they should proceed to restart from this slot.
Otherwise validators should halt and send alerts for human attention.

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

## Backwards Compatibility
This change is backward compatible with previous versions, because validators only
enter the new mode during new restart mode which is controlled by a command line
argument. All current restart arguments like  --wait-for-supermajority and
--expected-bank-hash will be kept as is for now.
However, this change does not work until at least 80% installed the new binary and
they are willing to use the new methods for restart.