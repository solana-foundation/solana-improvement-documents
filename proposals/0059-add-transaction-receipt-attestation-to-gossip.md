---
simd: '0059'
title: Add Transaction Receipt Attestation to Gossip
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

We propose a new variant in the CrdsData enum that stores
the receipt root, slot,an attestation to the root and slot
from the validator and the signer's public key.

```rust
pub struct ReceiptRoot{
  slot: u64 // The slot that the receipt is generated for
  signature: [u8;64], // A message signed by node with the root and slot as data
  root: [u8;32], // The transaction receipt root
  pubkey: [u8;32] // The public key of the validator
}
```

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

## Impact

CRDS will have receit roots which can be subscribed to by
light clients and this will be consistent across the entire cluster.
Verifying receipts by comparing the locally computed receipt with the cluster
wide receipt would be much more convenient.

WIP

## Security Considerations

WIP

## Drawbacks *(Optional)*

WIP
