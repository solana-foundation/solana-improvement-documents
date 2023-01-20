---
simd: '0010'
title: Diet Clients
authors:
  - Anatoly Yakovenko (Solana Labs)
category: Sealevel Virtual Machine
type: Standards Track
status: Draft
created: 2022-12-09
---

## Summary

Users that don't run a full node need secure access to the solana
blockchain. The goal of this proposal is to define a protocol for
sampling the validators such that if at least a small minority of
validators are not faulty the user can be notified of an invalid
state transition or a double confirmation attack on the network.

## Specification

### Overview

1. User observes a confirmation for a TX at slot N. Confirmation
contains 2/3+ votes for slot N, and a path from the transaction to
the bankhash signed by all the votes.

2. Concurrently with 1, user requests votes for slot N from M random
validators.  Votes returned may be for later slots, and would contain
a merkle path to the requested slot's bankhash. M should be large
enough to represent a small but significant percentage of the stake
above X%. X should  be configured by the client as the desired
threshold for confidence.

3. If all the responses are the same, the user is done. 

4. If X% or more of staked nodes respond that they do not have block,
the user starts sampling nodes at random for missing shreds and
submitting the shreds to the validators with the missing block.

    a. This process may need to run recursively until the fork block
    is repaired.

    b. If shreds cannot be recovered, a fault has occurred. User
    shouldn't trust the confirmation.

5. If X% or more of the staked nodes respond with different
bankhash, a fault has occurred on the network. User shouldn't trust
the confirmation. User should then submit the conflicting votes to
M random validators. User shouldn't trust any confirmations from
the network.

6. if X% or more of the staked nodes do not respond in T seconds a
faulty majority may have partitioned the network and the client.
Client should ignore the confirmation, manually confirm those nodes
are down, and remove them from the local view of quorum for this
transaction and try again. X should be configured large enough that
this doesn't happen more then once a year. Clients can automatically
remove nodes from the local view of the quorum which have not voted
for K slots.

### Confirmation Proof

#### BankHash

BankHash = Merkle(ShredHash, BlockHash, StateHash, Prev BankHash)

#### ShredHash: Data availability merkle tree

This tree is necessary for step 4 in the protocol. If enough nodes
respond that they do not have the data, the client should try to
repair the data for them. This prevents the faulty majority from
withholding the data from the rest of the network, so the rest can
identify an invalid state transition or a double confirmation.

Turbine, solana's block propagation protocol, transmits blocks in
small chunks called shreds. Shreds are transmitted in batches.
Each batch includes J data shreds and K erasure codes.

Upon executing the block, the validator should compute the merkle
root of all the shreds, including the erasure shreds. The merkle
root should be added to the BankHash, which is signed by validators
when voting.

When sampling for shreds, the user knows that the shreds are for
the proposed confirmed block because each shred has a merkle path
to the bankhash that was signed by all the validators.

#### StateHash: Transaction state transition merkle tree

Merkle root of all the outputs from all the transactions and the
status codes.

### Future work: Separating execution and fork choice

Eventually the solana protocol will separate fork choice from
computing all the state transitions for all the programs. Diet
clients would need to be modified to provide confirmation of execution
for users.

BankHash = (ShredHash, BlockHash, VoteStateHash, Prev BankHash)

VoteStateHash = Current TX State Results for votes only

ProgramStateHash = (Current TX State Results, Previous ProgramStateHash)

While validators vote only on the BankHash, users still need to
know the ProgramStateHash for the slot. 

Validators are not expected to compute the ProgramStateHash at the
same time as the BankHash, and the ProgramStateHash may lag.  But
at least once an epoch, all the validators must compute the EpochHash
for the previous epoch, which includes all the ProgramStateHashe from the
previous epoch.

Epoch Hash = Merkle root of all the accounts including the account
that stores ProgramStateHashes for the epoch

#### Overview

The original protocol is modified as follows:

2. Concurrently, user requests at random for votes from N different
nodes. Votes maybe for later slots, and would contain a path to the
requested slot's bankhash.
2.a User also requests signed ProgramStateHash attestations from
the staked validators.

If the validator signed an invalid ProgramStateHash, this signed
attestation can be used to slash them after the EpochHash has been
computed by the network.

### Future work: Minimized fraud proof

Currently, the above proposal requires a client to validate the
entire chain starting from a previously trusted snapshot up to the
invalid block. This proposal covers a minimized fraud proof that
allows the user to detect when the majority signed an invalid state
transition. User would only need enough resources to process the
votes and one transaction.

#### Overview

1. Track all the inputs into state transitions

Each input should have a merkle path to the original snapshot, or
if it has been modified, to an output from a previous transaction.

2. Merkelize all the txs and their inputs and outputs for each block

       StateHash
     h1       h2

  tx1  tx2  tx3 tx4

tx:
    inputs: all read account paths: path to snapshot | path to previous
            block output | path to intra-block tx output
    result: success or failure code
    outputs: all write accounts if result is successful, 0 otherwise

The tree should contain an ordered sequence of txs. Each tx is its
own tree that contains all the input paths to the accounts used for
execution and all the outputs. Each input can either be an
output from a snapshot root, or an output from a previous block,
or from a transaction in the same block.

3. User receives a path for their confirmed TX along with the
majority of votes signing the root.  An honest node will generate
a path for the same TX when processing the same block.  Either
inputs will be different, or the outputs.

4. If the input is invalid, ether an invalid value, or a valid but
not the most recent value, user or the honest validator will need
to recursively request a confirmation for the transaction that
generated the input.

    a. An invalid value for the input would have no path to a
    previously valid header.

    b. An honest node can provide the most recent input and its
    path to a signed header to prove that the faulty majority used
    a valid but older input.

5. If the output is different, user will need to run the program
locally and confirm the result.

Eventually user will be able to confirm the invalid state transition
locally. The amount resources necessary to execute the confirmation
should be on the order of executing 1 transaction, which generally
fits in a mobile browser sandbox.

#### Storage

Upon storing the output of a transaction, AccountsDB should also
record the merkle path of that output to the transaction. At the
end of each block, each transaction in ledger order is merklelized
into the StateHash.

#### Retrieval

When retrieving the inputs for execution, each input should have
been tagged with a path to the transaction that generated it. That
path, along the path to the root of the block that generated the
transaction can be retrieved concurrently during replay.
