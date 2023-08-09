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
---

## Summary

This proposal introduces two new concepts to the Solana runtime.

- Receipts, a deterministic encoding of state changes induced by a transaction;
- The receipt tree, a commitment scheme over all transaction receipts in a slot.

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
serialization functions for receipts are defined to be deterministic to prevent
malleability.

To succinctly detect receipt mismatches, this proposal further introduces a
commitment scheme based on a binary hash tree that is constructed once per slot.

## Design Goals

1. Receipts should be deterministic.
   Given a transaction T and ledger history leading up to it, serializing the
   receipt generated for T  should result in the same byte vector for all nodes
   in the network.
   *Rationale:* Determinism is required for cluster-wide consensus.

2. Receipts should not be required during block construction.
   *Rationale:* Future upgrades propose tolerating asynchronous replay during
   block construction. In other words, validators should be allowed to produce
   and distribute a block before replaying said block. It is impossible to
   introduce such a tolerance if receipts are mandatory components of blocks.

## Alternatives Considered

### Using TransactionStatusMeta

An alternative to introducing a new receipt type is reusing the transaction
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
PoH chain for each block (block hash). Expanding the PoH hash is the least
complex option as of today but is consequential for future upgrades.

Such a change would be incompatible with design goal 2 because it redefines
the PoH hash to additionally commit to execution results, instead of only
ledger content. Block producers are then forced to synchronously replay while
appending PoH ticks.

Furthermore, it significantly changes behavior in the event of an execution
disagreement (e.g. due to a difference in execution behavior between
validators). Mixing execution results into PoH forces execution disagreements to
result in a chain split.

## Detailed Design

### Transaction Receipt Specification

The transaction receipt must contain the following information related to the transaction:

- Signature
- Execution Status

The receipt would be a structure defined as:

```rust
pub struct Receipt{
    pub signature: [u8;64],
    pub status: u8 // 1 or 0 would determine the post execution status
}
```

### Receipt Tree Specification

The receipt tree is a binary merkle tree of receipts, where each node is a 32
byte hash of the Receipt data structure.

We construct a deterministic tree over a list of Receipts per slot with
the following properties:

- The tree needs to be strictly deterministic as any other cryptographic
  primitive to ensure that trees are exactly identical when individually
  constructed by different nodes.

- The order of the leaves should match the order of the list
  of receipts. When validators replay the transactions in a slot, they should aggregate
  the receipts in the same order as the transactions were in the block.

- For membership proofs and inclusion checks one should be
  able to provide a path from the leaf node (Receipt) to the root of the tree.
  The locally computed root is compared for equality.

```txt
Receipt tree with four receipts as leaf nodes [L0, L1, L2, L3]
where R0, R1, R2, R3 are the receipts and Nγ is the root.

       Nγ
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


Receipt tree with five receipts as leaf nodes [L0, L1, L2, L3, L4]
where R0, R1, R2, R3 are the receipts and Nζ is the root.
          Nζ
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
```

[Link to the specification](https://github.com/solana-foundation/specs/blob/main/core/merkle-tree.md)

#### Benchmarks

We have performed benchmarks comparing two merkle tree implementations,
the benchmark was done on 1 million leaves, each leaf consisted of a 64 byte
signature and a single byte status.

1) Solana Labs Merkle Tree: This is the pure rust implementation that is currently
   used by the Solana Labs client.
   More details [Solana labs repository](https://github.com/solana-labs/solana/tree/master/merkle-tree)
2) Firedancer binary merkle tree (bmtree): Implemented in C and uses firedancer's
   optimised SHA-256 implementation as it's hashing algorithm. However the benchmarks
   were performed using its rust FFI bindings.
   More details: [Firedancer](https://github.com/firedancer-io/firedancer/tree/main/src/ballet/bmtree)
   ![Benchamrk Results](https://github.com/tinydancer-io/solana-improvement-documents/assets/50767810/6c8d0013-1d62-4c7b-8264-4ec71ea28d7c)

More details with an attached flamegraph can be found in our [repository](https://github.com/tinydancer-io/merkle-bench).

## Security Considerations

We prepend 0x0 to leaf nodes and 0x1 to internal nodes to avoid second
preimage attacks where a proof is provided with internal nodes as leaf nodes.

We also make sure that the proof doesn't contain any consecutive identical hashes.

Security considerations defined in the merkle tree specification
by the Firedancer team:

No practical collision attacks against SHA-256 are known as of Oct 2022.

Collision resistance is vital to ensure that the graph of nodes remains acyclic
and that each hash unambiguously refers to one logical node.

## Backwards Compatibility

Not applicable.
