---
simd: '0059'
title: Add Receipt Root to Gossip
authors:
  - Anoushk Kharangate (Tinydancer)
  - Harsh Patel (Tinydancer)
category: Standard
type: Networking
status: Draft
created: (2023-06-27)
---

## Summary

This SIMD introduces a new variant in the CrdsData enum called ReceitRoot.
It uses the new receipt tree commitment scheme introduced in [SIMD-0058](https://github.com/firedancer-io/solana-improvement-documents/blob/ripatel/transaction-receipts/proposals/0058-transaction-receipts.md?plain=1)

## Motivation

As discussed in [SIMD-0052](https://github.com/tinydancer-io/solana-improvement-documents/blob/main/proposals/0052-consensus-and-transaction-proof-verification.md)
there is a need for a user to validate certain information regarding their
transaction without trusting the RPC.
Therefore there needs to be a protocol for validators to attest that they have
signed on a block with certaintransaction and their respective exectuion results
 while also making it easily accessible and verifable for end users.

This would allow us to have light clients that can verifying simple transactions
without trusting the RPC providers and has been a important missing utility in
the ecosystem.

## Alternatives Considered

### Modifying Blockhash and Bankhash

We also ideated the possibility of using blockhash and by its extension the bankhash.

For every block that is created validators construct a blockhash that is a hash of
all the entries, each entry creates a merkle root of transactions in the block.
The blockhash is hashed with other information like account changes and the bankhash
is generated. The validators then vote on it and we can use the bankhash and blockhash
to verify if a transaction was included. However this is not enough and for a good
client user experience we would need to include the execution status and logs.
We would also need to create a merkle tree of all entries instead of a sequential
hash to make verification simple.

This has certain problems:

- It requires change the core consensus design of the network
- There is added overhead to adding statuses, logs and creating an entry merkle.
- It introduces challenges in development of stateless leaders due to dependence
  on execution state.

## New Terminology

ReceiptRoot - A structure containing the slot, transaction receipt
root and the signature of the validator attesting to the slot and receipt root.

## Detailed Design

We propose a new structure in the Gossip CRDS that stores
the receipt root, slot,an attestation to the root and slot
from the validator and the signer's public key.

This struct would be called ReceiptRoot and to store commitments
for multiple slots we would push them into BatchReceiptRoots.

```rust
pub struct BatchReceiptRoot{
  roots: [ReceiptRoot; N] // an array of upto N commitments
}

pub struct ReceiptRoot{
  slot: u64 // The slot that the receipt is generated for
  signature: [u8;64], // A message signed by node with the root and slot as data
  root: [u8;32], // The transaction receipt root
  pubkey: [u8;32] // The public key of the validator
}
```

Each node would push the batch every N slots and then every node in the network
especially RPCs can aggregate these and serve them to light clients.

### Eviction Policy

The standard eviction policy for a gossip entry is to store one record per node.
However it would be problematic in our case since as one record holds only one
commitment and the duration for a gossip entry (time taken to push and pull) can
easily exceed the slot time of 400ms. The following options offer potential solutions
but it essentially comes down to a tradeoff between gossip bandwidth and memory.

- Use a custom eviction policy (store last 30 records per node)
- Increase the poll rate of gossip to push and pull records more frequently
  (increases bandwidth)
- Batch multiple receipt roots in one message (increases latency)

Our research suggests that deploying the custom eviction policy and batching roots
is a good choice given that validators reserve instances with high ram upfront and
solana nodes are notorious for taking up more bandwidth due to gossip.

### Why do we need BatchReceiptRoot?

The gossip protocol pushes a new message every 7500ms, the time taken by a slot
is 400ms, if we were to push a new message for every slot then that would
consume more bandwidth.

Additionally the CRDS is not designed to handle multiple of the same record types
for a particular node hence it doesn't allow us to store records for as long as
we need to.

Note: We think batching 9 slots at a time is good considering that's only 6 seconds
and the max packet size is 1232 bytes(1280 with metadata) before it starts
getting fragmented over ethernet.

### Performance and Memory

ReceiptRoot - `8 + 64 + 32 + 32 = 136 Bytes`
BatchReceiptRoot - `136 * 9 = 1224 Bytes (1.224 kB)`

Since we are not changing the poll rate of gossip to be in sync with
the latest slot we would save on the gossip egress bandwidth and there
wouldn't be any significant changes in node operations and costs.

## Impact

This would enable SPV light clients that can locally verify confirmation of their
transaction without blindly trusting the RPC server and would greatly improve
the security and decentralization of the solana network.

## Security Considerations

While this SIMD greatly reduces the user's trust in an RPC, the light client will
still need to make certain trust assumptions. This includes finding trusting that
all transactions are valid (in case the supermajority is corrupt).
We plan to solve these problems in future SIMDs to provide a full trustless setup
including data availability sampling and fraud proving which will only require a
single honest full node.

Additonally, the advantage of using gossip is that not changing the consensus
commitment scheme doesn't risk bringing liveness failures to the network in the
future.

## Drawbacks

Currently there is no mechanism built in the spec to incentivise making commitments
and nodes may choose not to, however this is something that can be introduced
in the future.
