---
simd: '0023'
title: Asynchronous Program Execution and Broadcast
authors:
  - Anatoly Yakovenko
category: Standard/Meta
type: Core
status: Draft
created: 2024-04-29
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This feature changes how the ledger is broadcast and executed. It
separates proposing blocks full of user transactions from blocks
with votes.  It allows for N concurrent builders of user transaction
blocks, and it allows for asynchronous execution of user blocks.

## Motivation

1. Single leader for user transactions is a bottleneck.  Leaders
are in a single spot in the world, while clients are all over the
world. Latency is therefore based on how close a client is to a
leader.  Leaders also have to spend a ton of resources on prioritization
of transactions.

2. Executing programs before voting is a bottleneck. Fork choice
doesn't depend on program execution, and delaying voting creates
forks and also delays the next block producer from starting.

## Alternatives Considered

APEX design that was proposed in https://github.com/solana-labs/solana/pull/24127

## New Terminology

* UserBlock - a block full of none vote transactions.

* UserBlockEntry - a hash of the block that was proposed

* UserBlockSlot - a slot measuring half a normal slot in ticks.

* Leader - the current leader for the slot that will propose a PoH
ledger full of Votes and UserBlockEntry

* Builder - a node that is scheduled to propose a block with non
vote transactions

* VoteHash - like BankHash, but for vote only transactions.

* BundleTransactions - a transaction signed by the builder that can
list transactions in the buildres proposed block to be executed in
one ordered batch.  This transaction can also add a priority fee
such that the entire bundle is prioritized for execution as a batch.

## Detailed Design

### Overview
Leaders are scheduled to propose blocks as they are currently by
the LeaderSchedule.

Builder's are scheduled along side leaders to build blocks. N
number of builders can be scheudled per slot.  Builder are
scheudled at 2x the rate of leaders.  So for every leader slot,
there are N builders producing 2 UserBlocks.

While a leader is scheduled, they receive and encode votes as normal.
Any well formed UserBlocks that were received from the previous or
current UserBlockSlot are added to the leaders PoH ledger as
UserBlockEntry.

The N concurrent Builder have 200ms slots to create blocks out
of user transactions. These are transmitted to the network.  The
leader receives and decodes them and generates a UserBlockEntry,
and adds it to PoH as soon as the leaders PoH has started the
UserBlockSlot.

### Fork Choice

If a validator doesn't have the builder's UserBlock, the validator
doesn't vote on the proposed block and tries to repair the UserBlock.
That fork is ignored for fork choice.

Otherwise the validator evaluates the forks as usual. 

### UserBlock execution

Each UserBlock is assumed to have been created simultaneously during
the UserBlockSlot that it was encoded by the leader. For each
UserBlock, the transactions are ordered by priority fee before
execution. If two transactions from two different blocks have the
same priority, the earliest transaction by UserBlock is executed
first, otherwise by which UserBlock appears first in the leaders
PoH.

If the leader's proposed block is attached to the heaviest fork,
the validator can start execution of the UserBlocks optimistically.
Otherwise the validator should only execute UserBlocks on the
heaviest fork.

### UserBlock format

UserBlock follows the standard solana ledger format, except it
doesn't contain any PoH ticks.

### UserBlock broadcast

Builders can form blocks at any time. Each scheduled builder can
transmit shreds over a pre-allocated shred range for the slot.

For example, if the network is configured with 5k shred blocks, and
2 builders:

Leader: [0-1000]
UserBlockSlot 0, Builder 0: [1000-2000]
UserBlockSlot 0, Builder 1: [2000-3000]
UserBlockSlot 1, Builder 0: [3000-4000]
UserBlockSlot 1, Builder 1: [4000-5000]

Leader must fit their entire block in shreds 0 to 1000, including
PoH tick entries, votes and UserBlockEntries.

Builder 0 transmits shreds 1000-2000 and 3000-4000
Builder 1 transmits shreds 2000-3000 and 4000-5000

UserBlockEntries for UserBlockSlot 1 must be included by the leader
after the halfway point in the leaders block.

### BankHash

Votes must include the most recent computed BankHash along with the
slot for the BankHash computation.  This slot is different
from the tower vote slot.

Each validator that is computing a BankHash will be able to verify
the BankHash values created by block producers for previous slots
because each BankHash has a link to the previous one.

During verification, if a validator sees that > 1/3 of the network
has incorrectly computed a state transition, they should halt
immediately.

### VoteHash

All validators will execute all the vote transactions on every fork,
as they do now. After computing all the vote state transitions the
validator will compute the VoteHash, identical to the BankHash in
structure, but covers only the vote state transitions.  The VoteHash
is included in the vote for the slot that the validator is voting
for.

### Client Confirmations

Clients connected to a trusted RPC can confirm the transaction as
soon as the RPC has executed teh UserBlock and the block has been
optimistically confirmed.

Client can also wait for a status code weighted by stake weight,
as each validator's execution catches up to the slot and includes
a BankHash confirming the state of the execution.

It may be sufficient to wait for 1/3+ validators executing the state
transition instead of the full 2/3+, because if 1/3+ are incorrect
the network will halt anyways.

## Impact

Multiple nodes can operate as Builder on the network concurrently.
So clients can pick the nearest one, and the bandwidth to schedule
and prioritize transactions is doubled.  There needs to be a design
for BundleTransactions that allow the bundler to prioritize a batch
of transactions for execution by paying a priority fee for all of
them, and executing the whole batch together.

Priority fees now also imply execution priority.

Fork choice and voting is not blocked by user block propagation or
execution of user transactions.

Builders are going to capture all the MEV.

Votes contain additional data, instead of BankHash, they contain
(VoteHash, (BankHash, slot))

## Security Considerations

* Validators may include incorrect BankHash values which may not
be detected instantly.

* Validators may skip evaluating their own BankHash and copy results
from other validators.

* Network halts if it cannot compute a epoch snapshot hash once an
epoch because it is overloaded with user transactions.

* Leader could censor or delay the builder. 

* Under heavy forking, validators will skip executing all the
UserBlocks on minor forks.

## Economic Considerations

* Builders should be shuffled and scheduled according to stake weight.

## Drawbacks

[TBD]

## Backwards Compatibility

[TBD]
