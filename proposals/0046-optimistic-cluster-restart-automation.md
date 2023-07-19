---
simd: '0046'
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

During a cluster restart following an outage, make validators enter a separate
recovery protocol that uses gossip to exchange local status and automatically 
reach consensus on the block to restart from. Proceed to restart if validators
in the restart can reach agreement, or print debug information and halt
otherwise.

## New Terminology

* `cluster restart`: When there is an outage such that the whole cluster
stalls, human may need to restart most of the validators with a sane state so
that the cluster can continue to function. This is different from sporadic
single validator restart which does not impact the cluster. See
[`cluster restart`](https://docs.solana.com/running-validator/restart-cluster)
for details.

* `optimistically confirmed block`: a block which gets the votes from the
majority of the validators in a cluster (> 2/3 stake). Our algorithm tries to
guarantee that an optimistically confirmed will never be rolled back. When we 
are performing `cluster restart`, we normally start from the highest 
`optimistically confirmed block`, but it's also okay to start from a child of
the highest `optimistically confirmed block` as long as consensus can be
reached.

* `silent repair phase`: During the proposed optimistic `cluster restart`
automation process, the validators in restart will first spend some time to
exchange information, repair missing blocks, and finally reach consensus. The 
validators only continue normal block production and voting after consensus is
reached. We call this preparation phase where block production and voting are
paused the `silent repair phase`.

* `silent repair shred version`: right now we update `shred_version` during a
`cluster restart`, it is used to verify received shreds and filter Gossip
peers. In the proposed optimistic `cluster restart` plan, we introduce a new
temporary shred version in the `silent repair phase` so validators in restart
don't interfere with those not in restart. Currently this `silent repair shred
version` is calculated using `(current_shred_version + 1) % 0xffff`.

* `RESTART_STAKE_THRESHOLD`: We need enough validators to participate in a
restart so they can make decision for the whole cluster. If everything works
perfect, we only need 2/3 of the total stake. However, validators could die
or perform abnormally, so we currently set the `RESTART_STAKE_THRESHOLD` at
80%, which is the same as now.

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
may slowly try to automate more parts to improve cluster reliability.

### Use gossip and consensus to figure out restart slot before the restart

The main difference between this and the current restart proposal is this 
alternative tries to make the cluster automatically enter restart preparation 
phase without human intervention.

While getting humans out of the loop improves recovery speed, there are
concerns about recovery gossip messages interfering with normal gossip 
messages, and automatically start a new message in gossip seems risky.

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

1. The operator restarts the validator with a new command-line argument to
cause it to enter the `silent repair phase` at boot, where it will not make new 
blocks or vote. The validator propagates its local voted fork
information to all other validators in restart.

2. While aggregating local vote information from all others in restart, the
validator repairs all blocks which could potentially have been optimistically
confirmed.

3. After repair is complete, the validator counts votes on each fork and
sends out local heaviest fork.

4. Each validator counts if enough nodes can agree on one block (same slot and
hash) to restart from:

   1. If yes, proceed and restart

   2. If no, print out what it thinks is wrong, halt and wait for human

See each step explained in details below.

### 1. `silent repair phase`: Gossip last vote and ancestors on that fork

The main goal of this step is to propagate the last `n` ancestors of the last
voted fork to all others in restart.

We use a new Gossip message `LastVotedForkSlots`, its fields are:

* `last_voted_slot`: `u64` the slot last voted, this also serves as last_slot
for the bit vector.
* `last_voted_hash`: `Hash` the bank hash of the slot last voted slot.
* `ancestors`: `BitVec<u8>` compressed bit vector representing the slots on
sender's last voted fork. the most significant bit is always
`last_voted_slot`, least significant bit is `last_voted_slot-81000`.

The number of ancestor slots sent is hard coded at 81000, because that's
400ms * 81000 = 9 hours, we assume most restart decisions to be made in 9 
hours. If a validator restarts after 9 hours past the outage, it cannot join 
the restart this way. If enough validators failed to restart within 9 hours, 
then fallback to the manual, interactive `cluster restart` method.

When a validator enters restart, it uses `silent repair shred version` to avoid
interfering with those outside the restart. There is slight chance that 
the `silent repair shred version` would collide with the shred version after
the `silent repair phase`, but even if this rare case occurred, we plan to
flush gossip on successful restart before entering normal validator operation.

To be extra cautious, we will also filter out `LastVotedForkSlots` and
`HeaviestFork` in gossip if a validator is not in `silent repair phase`.

### 2. `silent repair phase`: Repair ledgers up to the restart slot

The main goal of this step is to repair all blocks which could potentially be
optimistically confirmed.

We need to prevent false negative at all costs, because we can't rollback an 
`optimistically confirmed block`. However, false positive is okay. Because when
we select the heaviest fork in the next step, we should see all the potential 
candidates for optimistically confirmed slots, there we can count the votes and
remove some false positive cases.

However, it's also overkill to repair every block presented by others. When
`LastVotedForkSlots` messages are being received and aggregated, a validator
can categorize blocks missing locally into 2 categories: must-have and ignored.
Depending on the stakes of validators currently in restart, some slots with too
few stake can be safely ignored, while others will be repaired.

In the following analysis, we assume:

* `RESTART_STAKE_THRESHOLD` is 80%
* `MALICIOUS_SET` which is validators which can disobey the protocol, is 5%.
   For example, these validators can change their votes from what they
   previously voted on.
* `OPTIMISTIC_CONFIRMED_THRESHOLD` is 67%, which is the percentage of stake
   required to be a `optimistically confirmed block`.

At any point in restart, let's call percentage of validators not in restart
`PERCENT_NOT_IN_RESTART`. We can draw a line at
`OPTIMISTIC_CONFIRMED_THRESHOLD` - `MALICIOUS_SET` - `PERCENT_NOT_IN_RESTART`.

Any slot above this line should be repaired, while other slots can be ignored
for now.

If
`OPTIMISTIC_CONFIRMED_THRESHOLD` - `MALICIOUS_SET` - `PERCENT_NOT_IN_RESTART`
is less than 10%, then the validators don't have to start any repairs.

We obviously want to repair all blocks above `OPTIMISTIC_CONFIRMED_THRESHOLD`
before the restart. The validators in `MALICIOUS_SET` could lie about their
votes, so we need to be conservative and lower the line accordingly. Also,
we don't know what the validators not in restart have voted, so we need to
be even more conservative and assume they voted for this block. Being
conservative means we might repair blocks which we didn't need, but we will
never miss any block we should have repaired.

For example, when only 5% validators are in restart, `PERCENT_NOT_IN_RESTART`
is 100% - 5% = 95%.
`OPTIMISTIC_CONFIRMED_THRESHOLD` - `MALICIOUS_SET` - `PERCENT_NOT_IN_RESTART`
= 67% - 5% - 95% < 10%, so no validators would repair any block.

When 70% validators are in restart, `PERCENT_NOT_IN_RESTART`
is 100% - 70% = 30%.
`OPTIMISTIC_CONFIRMED_THRESHOLD` - `MALICIOUS_SET` - `PERCENT_NOT_IN_RESTART`
= 67% - 5% - 30% = 32%, so slots with above 32% votes in `LastVotedForkSlots`
would be repaired.

When 80% validators are in restart, `PERCENT_NOT_IN_RESTART`
is 100% - 80% = 20%.
`OPTIMISTIC_CONFIRMED_THRESHOLD` - `MALICIOUS_SET` - `PERCENT_NOT_IN_RESTART`
= 67% - 5% - 20% = 42%, so slots with above 42% votes in `LastVotedForkSlots`
would be repaired.

From above examples, we can see the "must-have" threshold changes dynamically 
depending on how many validators are in restart. The main benefit is that a
block will only move from "must-have" to "ignored" as more validators 
join the restart, not vice versa. So the list of blocks a validator needs to
repair will never grow bigger when more validators join the restart.

Once the validator gets LastVotedForkSlots, it can draw a line which are the
"must-have" blocks. When all the "must-have" blocks are repaired and replayed,
it can proceed to step 3.

### 3. `silent repair phase`: Gossip current heaviest fork

The main goal of this step is to "vote" the heaviest fork to restart from.

We use a new Gossip message `HeaviestFork`, its fields are:

* `slot`: `u64` slot of the picked block.
* `hash`: `Hash` bank hash of the picked block.
* `stake_committed_percent`: `u16` total percentage of stakes of the validators
it received `HeaviestFork` messages from.

After receiving `LastVotedForkSlots` from the validators holding stake more 
than  `RESTART_STAKE_THRESHOLD` and repairing slots in "must-have" category,
replay all blocks and pick the heaviest fork as follows:

1. For all blocks with more than 67% stake in `LastVotedForkSlots` messages,
   they must be on the heaviest fork.

2. If a picked block has more than one child, check if the heaviest child
   should be picked using the following rule:

   1. If vote_on_child + stake_on_validators_not_in_restart >= 62%, pick child.
      For example, if 80% validators are in restart, child has 42% votes, then
      42 + (100-80) = 62%, pick child. 62% is chosen instead of 67% because 5%
      could make the wrong votes.

   It's okay to use 62% here because the goal is to prevent false negative
   rather than false positive. If validators pick a child of optimistically
   confirmed block to start from, it's okay because if 80% of the validators
   all choose this block, this block will be instantly confirmed on the chain.

   2. Otherwise stop traversing the tree and use last picked block.

After deciding heaviest block, gossip
`HeaviestFork(X.slot, X.hash, committed_stake_percent)` out, where X is the
latest picked block. We also gossip stake of received `HeaviestFork` messages
so that we can proceed to next step when enough validators are ready.

### 4. Exit `silent repair phase`: Restart if everything okay, halt otherwise

All validators in restart keep counting the number of `HeaviestFork` where
`received_heaviest_stake` is higher than `RESTART_STAKE_THRESHOLD`. Once a
validator counts that `RESTART_STAKE_THRESHOLD` of the validators send out
`HeaviestFork` where `received_heaviest_stake` is higher than
`RESTART_STAKE_THRESHOLD`, it starts the following checks:

* Whether all `HeaviestFork` have the same slot and same bank Hash. Because
validators are only sending slots instead of bank hashes in 
`LastVotedForkSlots`, it's possible that a duplicate block can make the
cluster unable to reach consensus. So bank hash needs to be checked as well.

* The voted slot is equal or a child of local optimistically confirmed slot.

If all checks pass, the validator immediately starts generation of snapshot at
the agreed upon slot.

While the snapshot generation is in progress, the validator also checks to see
whether two minutes has passed since agreement has been reached, to guarantee 
its `HeaviestFork` message propagates to everyone, then proceeds to restart:

1. Issue a hard fork at the designated slot and change shred version in gossip.
2. Execute the current tasks in --wait-for-supermajority and wait for
   `RESTART_STAKE_THRESHOLD` of the total validators to be in ready state.

Before a validator enters restart, it will still propagate `LastVotedForkSlots`
and `HeaviestFork` messages in gossip. After the restart,its shred_version will 
be updated so it will no longer send or propagate gossip messages for restart.

If any of the checks fails, the validator immediately prints out all debug info,
sends out metrics so that people can be paged, and then halts.

## Impact

This proposal adds a new silent repair mode to validators, during this phase
the validators will not participate in normal cluster activities, which is the
same as now. Compared to today's `cluster restart`, the new mode may mean more
network bandwidth and memory on the restarting validators, but it guarantees
the safety of optimistically confirmed user transactions, and validator 
operators don't need to manually generate and download snapshots again. 

## Security Considerations

The two added gossip messages `LastVotedForkSlots` and `HeaviestFork` will only
be sent and processed when the validator is restarted in the new proposed 
optimistic `cluster restart` mode. They will also be filtered out if a
validator is not in this mode. So random validator restarting in the new mode
will not bring extra burden to the system.

Non-conforming validators could send out wrong `LastVotedForkSlots` and
`HeaviestFork` messages to mess with `cluster restart`s, these should be
included in the Slashing rules in the future.

## Backwards Compatibility

This change is backward compatible with previous versions, because validators
only enter the new mode during new restart mode which is controlled by a
command line argument. All current restart arguments like
--wait-for-supermajority and --expected-bank-hash will be kept as is for now.
