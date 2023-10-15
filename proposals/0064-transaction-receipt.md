---
simd: '0064'
title: Transaction Receipts
authors:
  - Anoushk Kharangate (Tinydancer)
  - Harsh Patel (Tinydancer)
  - Richard Patel (Jump Firedancer)
category: Standard
type: Core
status: Draft
created: 2023-06-20
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal introduces two new concepts to the Solana runtime.

- TransactionReceiptData, a deterministic encoding of state changes induced by
  a transaction;
- Transaction Receipt Tree, a commitment scheme over all transaction receipts
  in a block.
  
## New Terminology

TransactionReceiptData: A deterministic encoding of state changes induced by a
transaction that includes the version, the status and a message hash of the transaction.

Transaction Receipt Tree: A commitment scheme over all transaction receipts
in a slot.

## Motivation

One of the fundamental requirements of a Solana client is access to the status
of a confirmed transaction. This is due to the fact that transactions are not
guaranteed to be confirmed once submitted, and may fail once executed. Virtually
all Solana clients therefore use a subscription-based or polling mechanism to
inquire whether a transaction was successfully executed.

The only standard mechanism to retrieve transaction statuses remotely is via the
RPC protocol. However, the RPC protocol only serves replay information of the
RPC service itself. It does not provide information whether the validator
network at large has derived the same information. This may allow an RPC
provider to send incorrect information to a client, such as marking a failed
transaction as successful.

Solana validator nodes replay all transactions and thus have access to
transaction status information. To improve security, clients should verify that
transaction status information received via RPC matches the validator network at
large.

This proposal introduces a new “transaction receipt” data structure, which
contains a subset of the information available via RPC. The derivation and
serialization functions for transaction receipts are defined to be deterministic
to prevent malleability.

To succinctly detect transaction receipt mismatches, this proposal further
introduces a commitment scheme based on a binary hash tree that is
constructed once per slot.

### Design Goals

1. Transaction Receipts should be deterministic.
   Given a transaction T and ledger history leading up to it, serializing the
   receipt generated for T  should result in the same byte vector for all nodes
   in the network.
   *Rationale:* Determinism is required for cluster-wide consensus.

2. Transaction Receipts should not be required during block construction.
   *Rationale:* Future upgrades propose tolerating asynchronous replay during
   block construction. In other words, validators should be allowed to produce
   and distribute a block before replaying said block. It is impossible to
   introduce such a tolerance if transaction receipts are mandatory components
   of blocks.

## Alternatives Considered

### Using TransactionStatusMeta

An alternative to introducing a new transation receipt type is reusing the transaction
status data as it appears in RPC ([TransactionStatusMeta]).  This would reduce
complexity for clients.

However, *TransactionStatusMeta* has no strict definition and is thus malleable
in violation of design goal 1. Technical reasons are as follows:

- Available fields vary between node releases.
- Log data is truncated based on node configuration.
- The [TransactionResult] type includes error codes, which are
  implementation-defined (Thus breaks multi-client compatibility).

This would make a commitment scheme impractical as-is. Addressing these concerns
is a breaking change.

  [TransactionStatusMeta]: https://docs.rs/solana-transaction-status/1.16.1/solana_transaction_status/struct.TransactionStatusMeta.html
  [TransactionResult]: https://docs.rs/solana-sdk/1.16.1/solana_sdk/transaction/type.Result.html

### Proof-of-History / Block Hash

Another alternative to introducing a new commitment scheme is reusing the
proof-of-history (PoH) hash chain. This was proposed in
[SPV Proposal](https://docs.solana.com/proposals/simple-payment-and-state-verification#transaction-merkle).

The PoH chain currently commits to the signatures of all transactions added to
the ledger. The consensus layer then periodically votes on the last state of the
PoH chain for each bankhash which is an extension of the blockhash.
Expanding the PoH hash is the least complex option as of today but
is consequential for future upgrades.

Such a change would be incompatible with design goal 2 because it redefines
the PoH hash to additionally commit to execution results, instead of only
ledger content. Block producers are then forced to synchronously replay while
appending PoH ticks.

Furthermore, it significantly changes behavior in the event of an execution
disagreement (e.g. due to a difference in execution behavior between
validators). Mixing execution results into PoH forces execution disagreements to
result in a chain split.

## Detailed Design

### TransactionReceiptData Specification

The transaction receipt must contain the following information related to the transaction:

- Version
- Message Hash
- Execution Status

The transaction receipt would be a structure defined as:

```rust
// Scalars are encoded in little-endian order

const RECEIPT_VERSION_V1: u64 = 1;

const RECEIPT_STATUS_SUCCESS: u8 = 0;
const RECEIPT_STATUS_FAILURE: u8 = 1;

// Size: 0x29
struct TransactionReceiptData {
    // Offset: 0x00
    // Must be RECEIPT_VERSION_V1
    version: u64,
    
    // Offset: 0x08
    message_hash: [u8;32],

    // Offset: 0x28
    // Must be one of RECEIPT_STATUS_{SUCCESS,FAILURE}
    status: u8,
}
```

### Transaction Receipt Tree Specification

The transaction receipt tree is a binary merkle tree of transaction receipts, where
each node is a 32 byte hash of the TransactionReceiptData data structure.

We construct a deterministic tree over a list of Transaction Receipts per slot with
the following properties:

- The tree needs to be strictly deterministic as any other cryptographic
  primitive to ensure that trees are exactly identical when individually
  constructed by different nodes.

- The order of the leaves should match the order of the list
  of transaction receipts. When validators replay the transactions in a slot,
  they should aggregate the transaction receipts in the same order as the
  transactions were in the block.

- For membership proofs and inclusion checks one should be
  able to provide a path from the leaf node (TransactionReceiptData) to the root
  of the tree. The locally computed root is compared for equality.
  
- Finally after aggregating all the transaction receipts and constructing the
  final root hash, the count of the transaction receipts is hashed with the root
  in 64-bit little endian byte ordering to produce a final commitment hash. This
  is done to prevent the possibility of length extension attack vector inherent to
  merkle trees where the total number of leaves is not fixed/known before
  tree construction.

```txt
Transaction receipt tree with an empty set of transaction receipts
where Nα is the root.
Nα := sha256(concat(0x80,0u64))

Transaction receipt tree with four transaction receipts as leaf
nodes [L0, L1, L2, L3] where R0, R1, R2, R3 are the transaction 
receipts and Nδ is the root.
           Nδ
          /  \
        /     \
       Nγ      N(transaction_receipts)
      /  \
     /    \
   Nα      Nβ
  / |     / \
 /  |    /   \
L0  L1   L2  L3

L0 := sha256(concat(0x00, R0))
L1 := sha256(concat(0x00, R1))
L2 := sha256(concat(0x00, R2))
L3 := sha256(concat(0x00, R3))
Nα := sha256(concat(0x01, hash(L0), hash(L1)))
Nβ := sha256(concat(0x01, hash(L2), hash(L3)))
Nγ := sha256(concat(0x01, hash(Nα), hash(Nβ)))
Nδ := sha256(concat(0x80, hash(Nγ),len([L0, L1, L2, L3])))

Transaction receipt tree with five transaction receipts 
as leaf nodes [L0, L1, L2, L3, L4] where R0, R1, R2, R3, R4 are the 
transaction receipts and Nτ is the root.
                Nτ   
              /   \
            /      \
          Nζ        N(transaction_receipts)
         /  \
        /    \
       Nδ     Iε
      /  \     \\
     /    \     \\
   Nα      Nβ    Nγ
  /  \    /  \   ||
 L0  L1  L2  L3  L4

L0 := sha256(concat(0x00, R0))
L1 := sha256(concat(0x00, R1))
L2 := sha256(concat(0x00, R2))
L3 := sha256(concat(0x00, R3))
L4 := sha256(concat(0x00, R4))
Nα := sha256(concat(0x01, hash(L0), hash(L1)))
Nβ := sha256(concat(0x01, hash(L2), hash(L3)))
Nγ := sha256(concat(0x01, hash(L4), hash(L4)))
Nδ := sha256(concat(0x01, hash(Nα), hash(Nβ)))
Nζ := sha256(concat(0x01, hash(Nδ), hash(Iε)))
Nτ := sha256(concat(0x80, hash(Nζ),len([L0, L1, L2, L3, L4])))

Here 'Nτ' is the root generated after concatenating
the node 'Nζ' with the length of vector of leaf nodes (which is five in the above
illustration) of the tree and hashing it.
```

[Link to the specification](https://github.com/solana-foundation/specs/blob/main/core/merkle-tree.md)

#### Using SHA-256

We have chosen SHA-256 over Blake3 as the hashing algorithm so we can take advantage
of the hardware optimisations like SHA-NI and FPGA made by the Jump Firedancer team.

This proposal reuses the Merkle tree construction used in other parts of the protocol.
Breaking changes to the construction, such as using a different hash function,
would introduce additional complexity to existing implementations.

#### Benchmarks

We have performed benchmarks comparing two merkle tree implementations,
the benchmark was done on 500k leaves, each leaf consisted of a 32 byte
message hash and a single byte status.

1) Solana Labs Merkle Tree: This is the pure rust implementation that is currently
   used by the Solana Labs client.
   More details [Solana labs repository](https://github.com/solana-labs/solana/tree/master/merkle-tree)
2) Firedancer binary merkle tree (bmtree): Implemented in C and uses firedancer's
   optimised SHA-256 implementation as it's hashing algorithm. However the benchmarks
   were performed using its rust FFI bindings.
   More details: [Firedancer](https://github.com/firedancer-io/firedancer/tree/main/src/ballet/bmtree)

![TINY_bench](https://github.com/tinydancer-io/solana-improvement-documents/assets/50767810/637dc83a-b3d2-4616-b70e-4fbb8a9e17fd)

More details with an attached flamegraph can be found in our [repository](https://github.com/tinydancer-io/merkle-bench).

## Impact

This would enable SIMD-0052 to be implemented where we add the transaction receipt
root to the bankhash. This would allow users to verify their transaction validity
and status without trusting the RPC.

## Security Considerations

We prepend 0x00 to leaf nodes, 0x01 to internal nodes and 0x80 to the root node respectively
to avoid second preimage attacks where a inclusion proof is provided with internal
nodes as leaf nodes.

We also make sure that the inclusion proof doesn't contain any consecutive
identical hashes.

Security considerations defined in the merkle tree specification
by the Firedancer team:

No practical collision attacks against SHA-256 are known as of Oct 2023.

Collision resistance is vital to ensure that the graph of nodes remains acyclic
and that each hash unambiguously refers to one logical node.

## Backwards Compatibility

Not applicable.
