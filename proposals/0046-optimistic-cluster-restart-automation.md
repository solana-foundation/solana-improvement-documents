---
simd: '0046'
title: Optimistic cluster restart automation
authors:
  - Wen Xu (Anza)
category: Standard
type: Core
status: Implemented
created: 2023-04-07
feature: N/A (gated by command line flag instead)
development:
  - Anza - implemented
  - Firedancer - implemented
---

## Summary

During a cluster restart following an outage, make validators enter a separate
recovery protocol that uses Gossip to exchange local status and automatically 
reach consensus on the block to restart from. Proceed to restart if validators
in the restart can reach agreement, or print debug information and halt
otherwise. To distinguish the new restart process from other operations, we
call the new process "Wen restart".

## New Terminology

* `cluster restart`: When there is an outage such that the whole cluster
stalls, human may need to restart most of the validators with a sane state so
that the cluster can continue to function. This is different from sporadic
single validator restart which does not impact the cluster. See
[`cluster restart`](https://docs.solana.com/running-validator/restart-cluster)
for details.

* `cluster restart slot`: In current `cluster restart` scheme, human normally
decide on one block for all validators to restart from. This is very often the
highest `optimistically confirmed block`, because `optimistically confirmed
block` should never be rolled back. But it's also okay to start from a child of
the highest `optimistically confirmed block` as long as consensus can be
reached.

* `optimistically confirmed block`: a block which gets the votes from the
majority of the validators in a cluster (> 2/3 stake). Our algorithm tries to
guarantee that an optimistically confirmed block will never be rolled back.

* `wen restart phase`: During the proposed optimistic `cluster restart`
automation process, the validators in restart will first spend some time to
exchange information, repair missing blocks, and finally reach consensus. The 
validators only continue normal block production and voting after consensus is
reached. We call this preparation phase where block production and voting are
paused the `wen restart phase`.

* `wen restart shred version`: right now we update `shred_version` during a
`cluster restart`, it is used to verify received shreds and filter Gossip
peers. In the proposed optimistic `cluster restart` plan, we introduce a new
temporary shred version in the `wen restart phase` so validators in restart
don't interfere with those not in restart. Currently this `wen restart shred
version` is calculated using `(current_shred_version + 1) % 0xffff`.

* `RESTART_STAKE_THRESHOLD`: We need enough validators to participate in a
restart so they can make decision for the whole cluster. If everything works
perfect, we only need 2/3 of the total stake. However, validators could die
or perform abnormally, so we currently set the `RESTART_STAKE_THRESHOLD` at
80%, which is the same as what we use now for `--wait_for_supermajority`.

## Motivation

Currently during a `cluster restart`, validator operators need to decide the
highest optimistically confirmed slot, then restart the validators with new
command-line arguments.

The current process involves a lot of human intervention, if people make a
mistake in deciding the highest optimistically confirmed slot, it is
detrimental to the viability of the ecosystem.

We aim to automate the negotiation of highest optimistically confirmed slot and
the distribution of all blocks on that fork, so that we can lower the 
possibility of human mistakes in the `cluster restart` process. This also
reduces the burden on validator operators, because they don't have to stay
around while the validators automatically try to reach consensus, the validator
will halt and print debug information if anything goes wrong, and operators can
set up their own monitoring accordingly.

However, there are many ways an automatic restart can go wrong, mostly due to
unforseen situations or software bugs. To make things really safe, we apply
multiple checks during the restart, if any check fails, the automatic restart
is halted and debugging info printed, waiting for human intervention. Therefore
we say this is an optimistic cluster restart procedure.

## Alternatives Considered

### Automatically detect outage and perform `cluster restart`

The reaction time of a human in case of emergency is measured in minutes,
while a `cluster restart` where human initiate validator restarts takes hours.
We considered various approaches to automatically detect outage and perform
`cluster restart`, which can reduce recovery speed to minutes or even seconds.

However, automatically restarting the whole cluster seems risky. Because
if the recovery process itself doesn't work, it might be some time before
we can get human's attention. And it doesn't solve the cases where new binary
is needed. So for now we still plan to have human in the loop.

After we gain more experience with the restart approach in this proposal, we
may slowly try to make the process more automatic to improve reliability.

### Use Gossip and consensus to figure out restart slot before the restart

The main difference between this and the current restart proposal is this 
alternative tries to make the cluster automatically enter restart preparation 
phase without human intervention.

While getting humans out of the loop improves recovery speed, there are
concerns about recovery Gossip messages interfering with normal Gossip 
messages, and automatically start a new message in Gossip seems risky.

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
blocks and the same set of last votes, so that they will with high probability
converge on the same canonical fork and proceed.

When the cluster is in need of a restart, we assume validators holding at least
`RESTART_STAKE_THRESHOLD` percentage of stakes will enter the restart mode.
Then the following steps will happen:

1. The operator restarts the validator into the `wen restart phase` at boot,
where it will not make new blocks or vote. The validator propagates its local
voted fork information to all other validators in restart.

2. While aggregating local vote information from all others in restart, the
validator repairs all blocks which could potentially have been optimistically
confirmed.

3. After enough validators are in restart and repair is complete, the validator
counts votes on each fork and computes local heaviest fork.

4. A coordinator which is configured on everyone's command line sends out its
heaviest fork to everyone.

5. Each validator verifies that the coordinator's choice is reasonable:

   1. If yes, proceed and restart

   2. If no, print out what it thinks is wrong, halt and wait for human

See each step explained in details below.

We assume that as most 5% of the validators in restart can be malicious or
contains bugs, this number is consistent with other algorithms in the consensus
protocol. We call these `non-conforming` validators.

### Wen restart phase

1. **Gossip last vote and ancestors on that fork**

   The main goal of this step is to propagate most recent ancestors on the last
   voted fork to all others in restart.

   We use a new Gossip message `RestartLastVotedForkSlots`, its fields are:

   * `last_voted_slot`: `u64` the slot last voted, this also serves as
   last_slot for the bit vector.
   * `last_voted_hash`: `Hash` the bank hash of the slot last voted slot.
   * `ancestors`: `Run-length encoding` compressed bit vector representing the
   slots on sender's last voted fork. the least significant bit is always
   `last_voted_slot`, most significant bit is `last_voted_slot-65535`.

   The max distance between oldest ancestor slot and last voted slot is hard
   coded at 65535, because that's 400ms * 65535 = 7.3 hours, we assume that
   most validator administrators would have noticed an outage within 7 hours,
   and the optimistic confirmation must have halted within 64k slots of the
   last confirmed block. Also 65535 bits nicely fits into u16, which makes
   encoding more compact. If a validator restarts after 7 hours past the
   outage, it cannot join the restart this way. If enough validators failed to
   restart within 7 hours, then we fallback to the manual, interactive
   `cluster restart` method.

   When a validator enters restart, it uses `wen restart shred version` to
   avoid interfering with those outside the restart. To be extra cautious, we
   will also filter out `RestartLastVotedForkSlots` and `RestartHeaviestFork`
   (described later) in Gossip if a validator is not in `wen restart phase`.
   There is a slight chance that the `wen restart shred version` would collide
   with the shred version after the `wen restart phase`, but with the filtering
   described above it should not be a problem.

   When a validator receives `RestartLastVotedForkSlots` from someone else, it
   will discard all slots smaller than the local root. Because the local root
   should be an `optimistic confirmed` slot, it does not need to keep any slot
   older than local root.

2. **Repair ledgers up to the restart slot**

   The main goal of this step is to repair all blocks which could potentially
   be optimistically confirmed.

   We need to prevent false negative at all costs, because we can't rollback an
   `optimistically confirmed block`. However, false positive is okay. Because
   when we select the heaviest fork in the next step, we should see all the
   potential candidates for optimistically confirmed slots, there we can count
   the votes and remove some false positive cases.

   However, it's also overkill to repair every block presented by others. When
   `RestartLastVotedForkSlots` messages are being received and aggregated, a
   validator can categorize blocks missing locally into 2 categories: must-have
   and ignored.

   We repairs all blocks with no less than 42% stake. The number is
   `67% - 5% - stake_on_validators_not_in_restart`. We require that at least 80%
   join the restart, any block with less than 67% - (100 - 80)% - 5% = 42% can
   never be optimistically confirmed before the restart.
   
   It's possible that different validators see different 80%, so their
   must-have blocks might be different, but there will be another repair round
   in the final step so this is fine. Whenever some block gets to 42%, repair
   could be started, because when more validators join the restart, this number
   will only go up but will never go down.

   When a validator gets `RestartLastVotedForkSlots` from 80% of the stake, and
   all those "must-have" blocks are repaired, it can proceed to next step.

3. **Calculate heaviest fork**

   After receiving `RestartLastVotedForkSlots` from the validators holding
   stake more than `RESTART_STAKE_THRESHOLD` and repairing slots in "must-have"
   category, pick the heaviest fork like this:

   1. Calculate the threshold for a block to be on the heaviest fork, the
   heaviest fork should have all blocks with possibility to be optimistically
   confirmed. The number is `67% - 5% - stake_on_validators_not_in_restart`.

   For example, if 80% validators are in restart, the number would be
   `67% - 5% - (100-80)% = 42%`. If 90% validators are in restart, the number
   would be `67% - 5% - (100-90)% = 52%`.

   2. Sort all blocks over the threshold by slot number, and verify that they
   form a single chain. The first block in the list should be the local root.

   If any block does not satisfy above constraint, print the first offending
   block and exit.

   The list should not be empty, it should contain at least the local root.

   To see why the above algorithm is safe, we will prove that:

   1. Any block optimistically confirmed before the restart will always be
   on the list:

   Assume block A is one such block, it would have `67%` stake, discounting
   `5%` non-conforming and people not participating in wen_restart, it should
   have at least `67% - 5% - stake_on_validators_not_in_restart` stake, so it
   should pass the threshold and be in the list.

   2. Any block in the list should only have at most one child in the list:

   Let's use `X` to denote `stake_on_validators_not_in_restart` for brevity.
   Assuming a block has child `A` and `B` both on the list, the children's
   combined stake would be `2 * (67% - 5% - X)`. Because we only allow one
   RestartHeaviestFork per pubkey, every validator should select either `A`
   or `B`, it's easy to find and filter out vialators who selected both. So the
   children's total stake should be less than `100% - X`. We can calculate that
   if `124% - 2 * X < 100% - X`, then `X > 24%`, this is not possible when we
   have at least 80% of the validators in restart. So we prove any block in the
   list can have at most one child in the list by contradiction.

   3. If a block not optimistically confirmed before the restart is on the
   list, it can only be at the end of the list and none of its siblings are
   on the list.

   Let's say block D is the first not optimistically confirmed block on the
   list, its parent E is confirmed and on the list. We know from above point
   that E can only have 1 child on the list, therefore D must be at the end
   of the list while its siblings are not on the list.

   Even if the last block D on the list may not be optimistically confirmed,
   it already has at least `42% - 5% = 37%` stake. Say F is its sibling with
   the most stake, F can only have less than `42%` stake because it's not on
   the list. So picking D over F is equal to the case where `5%` stake
   switched from fork F to fork D, 80% of the cluster can switch to fork D
   if that turns out to be the heaviest fork.

   After picking the appropriate slot, replay the block and all its ancestors
   to get the bankhash for the picked slot.

4. **Verify the heaviest fork of the coordinator**

   There will be one coordinator specified on the command line of everyone's
   command line. Even though everyone will calculate its own heaviest fork in
   previous step, only the coordinator's heaviest fork will be checked and
   optionally accepted by others.

   We use a new Gossip message `RestartHeaviestFork`, its fields are:

   * `slot`: `u64` slot of the picked block.
   * `hash`: `Hash` bank hash of the picked block.

   After deciding the heaviest block, the coordinator Gossip
   `RestartHeaviestFork(X.slot, X.hash)` out, where X is the block the
   coordinator picked locally in previous step. The coordinator will stay up
   until manually restarted by its operator.

   For every non-coordinator validator, it will perform the following actions
   on the heaviest fork sent by the coordinator:

   1. If the bank selected is missing locally, repair this slot and all slots
   with higher stake.

   2. Check that the bankhash of selected slot matches the data locally.

   3. Verify that the selected fork contains local root, and that its local
   heaviest fork slot is on the same fork as the coordinator's choice.

   If any of the above repair or check fails, exit with error message, the
   coordinator may have made a mistake and this needs manual intervention.

   When exiting this step, no matter what a non-coordinator validator chooses,
   it will send a `RestartHeaviestFork` back to leader to report its status.
   This reporting is just for ease of aggregating the cluster's status at the
   coordinator, it doesn't have other effects.

5. **Generate incremental snapshot and exit**

If the previous step succeeds, the validator immediately starts adding a hard
fork at the designated slot and perform `set_root`. Then it will start
generating an incremental snapshot at the agreed upon `cluster restart slot`.
After snapshot generation completes, the `--wait_for_supermajority` args with
correct shred version, restart slot, and expected bankhash will be printed to
the logs.

After the snapshot generation is complete, a non coordinator then exits with
exit code `200` to indicate work is complete.

A coordinator will stay up until restarted by the operator to make sure any
late comers get the `RestartHeaviestFork` message. It also aggregates the
`RestartHeaviestFork` messages sent by the non-coordinators to report on the
status of the cluster.

## Impact

This proposal adds a new `wen restart` mode to validators, under this mode the
validators will not participate in normal cluster activities. Compared to
today's `cluster restart`, the new mode may mean more network bandwidth and
memory on the restarting validators, but it guarantees the safety of
optimistically confirmed user transactions, and validator operators don't need
to manually generate and download snapshots during a `cluster restart`. 

## Security Considerations

The two added Gossip messages `RestartLastVotedForkSlots` and
`RestartHeaviestFork` will only be sent and processed when the validator is
restarted in `wen restart` mode. So random validator restarting in the new
mode will not clutter the Gossip CRDS table of a normal system.

Non-conforming validators could send out wrong `RestartLastVotedForkSlots`
messages to mess with `cluster restart`s, these should be included in the
Slashing rules in the future.

### Handling oscillating votes

Non-conforming validators could change their last votes back and forth, this
could lead to instability in the system. We forbid any change of slot or hash
in `RestartLastVotedForkSlots` or `RestartHeaviestFork`, everyone will stick
with the first value received, and discrepancies will be recorded in the proto
file for later slashing.

### Handling multiple epochs

Even though it's not very common that an outage happens across an epoch
boundary, we do need to prepare for this rare case. Because the main purpose
of `wen restart` is to make everyone reach aggrement, the following choices
are made:

* Every validator only handles 2 epochs, any validator will discard slots
which belong to an epoch which is > 1 epoch away from its root. If a validator
has very old root so it can't proceed, it will exit and report error. Since
we assume an outage will be discovered within 7 hours and one epoch is roughly
two days, handling 2 epochs should be enough.

* The stake weight of each slot is calculated using the epoch the slot is in.
Because right now epoch stakes are calculated 1 epoch ahead of time, and we
only handle 2 epochs, the local root bank should have the epoch stakes for all
epochs we need.

* When aggregating `RestartLastVotedForkSlots`, for any epoch with validators
voting for any slot in this epoch having at least 33% stake, calculate the
stake of active validators in this epoch. Only exit this stage if all epochs
reaching the above bar has > 80% stake. This is a bit restrictive, but it
guarantees that whichever slot we select for HeaviestFork, we have enough
validators in the restart. Note that the epoch containing local root should
always be considered, because root should have > 33% stake.

Now we prove this is safe, whenever there is a slot being optimistically
confirmed in the new epoch, we will only exit the aggregating of
`RestartLastVotedForkSlots` stage if > 80% in the new epoch joined:

1. Assume slot `X` is optimistically confirmed in the new epoch, it has >67%
stake in the new epoch.

2. Our stake warmup/cooldown limit is at 9% currently, so at least
67% - 9% = 58% of the stake were from the old epoch.

3. We always have >80% stake of the old epoch, so at least
58% - 20% = 38% of the stake were in restart. Excluding non-conforming
stake, at least 38% - 5% = 33% should be in the restart and they
should at least report they voted for `X` which is in the new epoch.

4. According to the above rule we will require >80% stake in the new
epoch as well.

## Backwards Compatibility

This change is backward compatible with previous versions, because validators
only enter the new mode during new restart mode which is controlled by a
command line argument. All current restart arguments like
`--wait-for-supermajority` and `--expected-bank-hash` will be kept as is.
