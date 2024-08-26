---
simd: '0064'
title: Transaction Receipts
authors:
  - Anoushk Kharangate (Tinydancer)
  - Richard Patel (Jump)
  - Harsh Patel (Tinydancer)
category: Standard
type: Core
status: Stale
created: 2023-06-20
feature: N/A
---

## Summary

Here we propose a mechanism for proving transaction inclusion into a block in
the Solana protocol. This is a pre-requisite for several use-cases that would
like to build upon a [Simple Payment Verification](https://en.wikipedia.org/wiki/Bitcoin_network#Payment_verification)
like construction.

We employ the well-known [Merkle Tree](https://en.wikipedia.org/wiki/Merkle_tree)
data structure to compress a block's transactions and their results into a compact
identifier, with which inclusion proofs can be generated
  
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

1. Transaction Receipts must be deterministic.
   Given a transaction T and ledger history leading up to it, serializing the
   receipt generated for T  should result in the same byte vector for all nodes
   in the network.
   *Rationale:* Determinism is required for cluster-wide consensus.

2. Transaction Receipts must not be required during block construction.
   *Rationale:* Future upgrades propose tolerating asynchronous replay during
   block construction. In other words, validators should be allowed to produce
   and distribute a block before replaying said block. It is impossible to
   introduce such a tolerance if transaction receipts are mandatory components
   of blocks.

## Alternatives Considered

### Using TransactionStatusMeta

An alternative to introducing a new transaction receipt type is reusing the transaction
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

### TransactionReceiptData

A TransactionReceiptData v1 object deterministically encodes the effects of executing
a transaction.  It contains the following information:

- Transaction Receipt Version (currently v1, other values reserved for future upgrades)
- Message Hash identifying the transaction it refers to
- The Execution Status, a boolean identifying whether the transaction was successfully
  executed or failed without extrinsic state changes.

It is defined by the following pseudocode.

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

#### Binary layout

The layout of the transaction receipt data when being hashed is as follows:
0x00 Prefix
0x01 Version
0x09 Status
0x0a Message Hash

Since SHA-256 prefers 32 byte blocks with non-overlapping data, we considered padding
the data to be aligned. However, without precomputation this would be worse in
terms efficiency as it would require on block more. With precomputation the
efficiency would be roughly the same. Below are the two layouts:

**Naive Case:**

```txt
// SHA block 0
[0x00..0x01] Merkle Node Type
[0x01..0x09] Version
[0x09..0x0a] Status
[0x0a..0x20] Message Hash
// SHA block 1
[0x20..0x2a] Message Hash

[0x2a..0x33] Merkle-Damgard Suffix
[0x33..0x40] Merkle-Damgard Padding
```

**Potentially Optimised Case:**

```txt
// SHA block 0 (precomputed)
[0x00..0x01] Merkle Node Type
[0x01..0x09] Version
[0x09..0x0a] Status
[0x0a..0x20] Padding
// SHA block 1
[0x20..0x40] Message Hash
// SHA block 2
[0x40..0x49] Merkle-Damgard Suffix
[0x49..0x60] Merkle-Damgard Padding
```

The leaf is constructed by hashing the struct fields in this order:
hash(0x0, version, status, message_hash)

##### Key considerations

- When inserted as a leaf node into the tree defined below,
  this structure requires hashing two SHA-256 blocks.
- The `version` integer can safely be shortened to `u8` in
  a future upgrade (Due to little-endian encoding). Using a
  u64 allows for this flexibilty. Futhermore, we could also
  fit a domain hash separator in the `version` bits to prevent
  re-interpreting the leaf nodes as a different structure in
  the future as a good practice.
- In future versions, hashes for a subset of fields exhibiting
  a small set of possible combinations could be aligned to the
  hash function block size and precomputed. (Such as `version`
  and `status`). For now, this micro-optimization would yield
  no performance improvements as mentioned above.

### Transaction Receipt Tree

The transaction receipt tree is a binary merkle tree of transaction receipts, where
each node is a 32 byte hash.

The tree is derived from a vector of TransactionReceiptData objects.
It is designed to feature the following properties:

- The Merkle tree construction used is an extension of the tree construction used
  in PoH. (With different input data)

- The tree derivation function is surjective:
  Each vector of transaction receipts results in a unique tree,
  thereby making it deterministic and immalleable.

- The order of the leaf nodes matches the block's transaction order.
  (As it unambiguously appears in the PoH construction)

- Succinct inclusion proofs are constructed by providing a hash path
  from a leaf node to the root node. The inclusion proof format is defined
  separately from this SIMD.

#### Specification

- Input: Vector of TransactionReceiptData objects in PoH order

- Pre-condition: Each element's `message_hash` is unique

- Output: Transaction receipt tree root (32 byte hash)

- Definitions:
  - The `intermediate_root` is the 32 byte root of the binary merkle tree
    as externally specified.
    [Specification: Binary Merkle Tree](https://github.com/solana-foundation/specs/blob/main/core/merkle-tree.md)
  - The `root_prefix` is the byte `0x80`
  - The function `u64_le_encode` encodes a 64-bit integer in little-endian
    byte order.
  - The `leaf_count` is the number of TransactionReceiptData objects to
    serve as leaf nodes in the tree.
  - `sentinel_root` is a byte array of zeros with length 32

- If the leaf count is zero, the output is
  `sha256(root_prefix || sentinel_root || u64_le_encode(0))`.

- If the leaf count is non-zero, the output is
  `sha256(root_prefix || intermediate_root || u64_le_encode(leaf_count))`.

#### Tree design considerations

- *Use of the PoH hash tree construction*
  - Allows reusing existing code for constructing the transaction receipt tree.
  - Avoids introducing a new cryptographic construction.
  - Alternatives to SHA-256 based constructions, such as BLAKE3 or SHA-3 would
    offer superior theoretical performance.
  - Existing optimized SHA-256 hash tree implementations are readily available.
  - As of 2023-Oct, hardware implementations of SHA-256 are currently available
    (via SHA-NI on x86, via Firedancer's SystemVerilog implementation on AWS F1
    FPGA), whereas BLAKE3 and SHA-3 are not. (SHA-3 is only available on recent
    Arm cores)
  - As of 2023-Oct, the Merkle tree root for an empty input vector is
    unspecified, which is a specification bug. The canonical PoH Merkle tree
    in the Solana Labs implementation does not define the empty tree either.
    Therefore, we introduce zero as the sentinel value for the root of an empty
    hash tree.

- *Leaf count suffix*:
  The PoH tree implicitly expands the internal leaf count to a power of two,
  causing the `intermediate_root` two have more than one pre-image for certain
  leaf counts.  This avoided by instead including the leaf count suffix in the
  final hash.

- *Exclusion proofs are not provided*:
  Although it is possible to construct Merkle-based set exclusion proofs, this
  feature was not part of this proposal's design criteria.  A proposed exclusion
  proof mechanism involves sorting transaction receipts by their message hash.
  This change may be considered in a future version of the transaction receipt
  tree.

#### Examples

The following illustrates transaction receipt hash trees for various inputs.

```txt
Transaction receipt tree with an empty vector of transaction receipts
where Nα is the root.
Nα := sha256(0x80 || intermediate_root([0u8;32]) || u64_le_encode(0))

Nα
|
0
```

```txt
Transaction receipt tree with four transaction receipts as leaf
nodes [L0, L1, L2, L3] where R0, R1, R2, R3 are the serialized
TransactionReceiptData objects and Nδ is the root.

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
Nα := sha256(concat(0x01, L0, L1))
Nβ := sha256(concat(0x01, L2, L3))
Nγ := sha256(concat(0x01, Nα, Nβ))
Nδ := sha256(concat(0x80, Nγ, u64_le_encode(4)))  # leaf count
```

```txt
Transaction receipt tree with five transaction receipts 
as leaf nodes [L0, L1, L2, L3, L4] where R0, R1, R2, R3, R4 are the
serialized TransactionReceiptData objects and Nτ is the root.

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
Nα := sha256(concat(0x01, L0, L1))
Nβ := sha256(concat(0x01, L2, L3))
Nγ := sha256(concat(0x01, L4, L4))
Nδ := sha256(concat(0x01, Nα, Nβ))
Iε := sha256(concat(0x01, Nγ, Nγ))
Nζ := sha256(concat(0x01, Nδ, Iε))
Nτ := sha256(concat(0x80, Nζ, u64_le_encode(5)))  # leaf count
```

#### Benchmarks

Tinydancer has prepared benchmarks comparing two merkle tree implementations.
The input used was a vector of 500k TransactionReceiptData objects.

1) Solana Labs Merkle Tree: This is the pure Rust implementation that is currently
   used by the Solana Labs client.
   [Solana Labs GitHub](https://github.com/solana-labs/solana/tree/master/merkle-tree)
2) Firedancer binary merkle tree (bmtree): Implemented in C and uses Firedancer's
   optimised SHA-256 implementation. Benchmarks were performed via Rust FFI bindings,
   and thus may not be representative of the upper bound.
   An improved version using AVX-512 instructions is in progress.
   [Firedancer GitHub](https://github.com/firedancer-io/firedancer/tree/main/src/ballet/bmtree)

[Benchmark Source](https://github.com/tinydancer-io/merkle-bench)
![TINY_bench](https://github.com/tinydancer-io/solana-improvement-documents/assets/50767810/637dc83a-b3d2-4616-b70e-4fbb8a9e17fd)

## Impact

The generation of a Transaction Receipt Tree is a prerequisite for providing proof
that a transaction was included in a block. Itself a step toward providing proof
that a transaction was executed and accepted under consensus by a Solana cluster.
A major improvement in trust-minimization for the ecosystem, opening the door to
new use-cases and reducing infrastructure requirements for some of today's.

## Security Considerations

The transaction receipt tree is expected to be used for transaction inclusion proofs.
For example, an inclusion proof might attest that a token transfer has succeeded.
Such proofs may then be relied on to provide financial services.

It is thus important to ensure that proofs cannot be forged.

Common forgery attacks against Merkle trees include:

- Various forms of pre-image attacks against the underlying hash functions.
  As of 2023-Oct, no practical collision attacks against SHA-256 are known.
- Malleability and type confusion in the hash tree construction.
  These are prevented via two mechanisms:
  1. Hash domain separation via one byte prefixes (for leaf nodes, branch
     nodes, and the final hash respectively)
  2. A node count suffix to prevent malleable leaf count attacks

Further conerns include:

- Implementation bugs: To reduce the risk of such, this proposal deliberately
  keeps the amount of newly introduced logic low.
- Performance-related attacks (DoS): The computational complexity of transaction
  receipt tree construction is `O(n+log n)` where `n` is the number of
  transactions. There are no other user controllable components such as
  variable-length inputs.

## Backwards Compatibility

This change does not impact the Solana ledger, and thus introduces no backwards
compatibility concerns.
