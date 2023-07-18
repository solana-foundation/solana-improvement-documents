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

3. Construction of receipt commitments should initially be optional to
   validators.
   *Rationale:* Enforcing construction of receipt commitments (e.g.
   by slashing validators that don’t) introduces additional security
   considerations. The failure domain of additional receipt logic should be
   isolated in the initial rollout to allow for timely activation.

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

### Bank Hash

An alternative to introducing a new commitment scheme is reusing the bank hash.

When executing transactions, Solana validators only indirectly commit to the
state changes via the bank hash. Namely, the bank hash commits to all changed
accounts after replaying a slot using a binary hash tree. However, it does not
commit to intermediate states during replay.

Redefining the bank hash to use a construction with transaction-level
granularity is a breaking change. Because construction of the bank hash is
practically mandatory for validators, it also violates design goal 3.

### Proof-of-History / Block Hash

Another alternative to introducing a new commitment scheme is reusing the
proof-of-history (PoH) hash chain. This was proposed in
[SIMD-0052](https://github.com/solana-foundation/solana-improvement-documents/pull/52).

The PoH chain currently commits to the signatures of all transactions added to
the ledger. The consensus layer then periodically votes on the last state of the
PoH chain for each block (block hash). Expanding the PoH hash is the least
complex option as of today but is consequential for future upgrades.

Such a change would be incompatible with design goal 2 (and by extension, goal
3) because it redefines the PoH hash to additionally commit to execution
results, instead of only ledger content. Block producers are then forced to
synchronously replay while appending PoH ticks.

Furthermore, it significantly changes behavior in the event of an execution
disagreement (e.g. due to a difference in execution behavior between
validators). Mixing execution results into PoH forces execution disagreements to
result in a chain split.

## Detailed Design

### Transaction Receipt Specification

The transaction receipt must contain the following information related to the transaction:

- Signature
- Execution Status
- Truncated Logs

The receipt would be a structure defined as:

```rust
pub struct Receipt{
    pub signature: [u8;64],
    pub status: u8 // 1 or 0 would determine the post execution status
    pub logs: [String;50] 
}
```

The logs are important to verify the stakeweights of validators,
[SIMD-0056](https://github.com/solana-foundation/solana-improvement-documents/pull/56)
introduces a program that provides on-chain access to validator stake weights.
This is important for light clients to verify that the validators that vote on a
certain block have greater than X% stake. To achieve this, the client sends a transaction
that logs the stake weights, it then requests for a proof from the receipt
to the bankhash. We can do this because the receipt includes the logs and the validators
vote on the bankhash.

Due to performance and compute concerns the logs would be truncated to a max of 50,
we can make this limit dynamic and charge more CU for more logs but that would
be a separate change.

### Receipt Tree Specification
```
Receipt tree with four receipts as leaf nodes [R1, R2, R3, R4] where R1, R2, R3 and R4 are the  
receipts of transactions 1, 2, 3, and 4.

        R
      /  \
     /    \
   Ri      Ri'
  / |     / \
 /  |    /   \
R1  R2   R3  R4

```
