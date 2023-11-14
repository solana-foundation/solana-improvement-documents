---
simd: '0052'
title: Add Receipt Root to Bankhash
authors:
  - Anoushk Kharangate (Tinydancer)
  - Harsh Patel (Tinydancer)
  - x19
category: Standard
type: Core
status: Draft
created: 2023-05-30
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal describes the inclusion of a transaction receipt tree root, 
which is a 32 byte SHA-256 hash, to the current Bankhash construction. 
This will enable a way to verify transaction inclusion by a merkle proof 
which has a path from the transaction receipt to the transaction receipt 
tree root since the network votes on the bankhash for consensus. 

## Motivation

The transaction receipt tree proposes a commitment scheme to construct 
a binary merkle tree of transaction receipts which contain information 
about the execution status of the transaction. The root of this tree 
needs to have network wide consensus per slot. Hence we propose 
adding the transaction receipt root to Bankhash since it's voted 
on by the staked nodes who participate in consensus. This will 
help Solana clients verify transaction inclusion and supermajority 
consensus on the transaction hence enabling consensus light clients. 

## Alternatives Considered

None

## New Terminology

Transaction Receipt Root: The merkle root hash of a binary merkle tree of 
TransactionReceiptData described in [SIMD-0064](https://github.com/solana-foundation/solana-improvement-documents/pull/64).

## Detailed Design

### Modifying the Bankhash

We propose the following change:

We add a transaction receipt root to the bankhash calculation where the receipt
root is the root of the merkle tree of transaction receipts. 
This root would be a sha256 hash constructed as a final result of the 
binary merkle tree of receipts. Specifically it will be a 32 byte array. 
The receipt root would be added to the bankhash as follows:

   ``` rust
   let mut hash = hashv(&[
      parent_hash,
      accounts_delta_hash,
      receipt_root,
      signature_count_buf,
      last_blockhash,
   ]);
   ```

## Impact

This proposal will improve the overall security and decentralization of the Solana
network allowing users to access the blockchain in a trust minimized way unlike
traditionally where users had to fully trust their RPC providers. Dapp developers
don't have to make any changes as wallets can easily integrate the client making
it compatible with any dapp.

The proposal would also be compatible with the future protocol updates like
Bankless leaders since the tree construction would be done async by buffering
transaction statuses. Bankless leaders won't need replay before propagating
the block.

## Security Considerations

### Trust Assumptions and Future Work

While this SIMD greatly reduces the user's trust in an RPC, the light client will
 still need to make certain trust assumptions. This includes finding a trusted
 source for the validator set per epoch (including their pubkeys and stake weights)
 and trusting that all transactions are valid (in case the supermajority is corrupt).
 We plan to solve these problems in future SIMDs to provide a full trustless setup
 including data availability sampling and fraud proving which will only require a
 single honest full node.

## Backwards Compatibility

The change is not backwards compatible due to which it would require
a feature flag activation strategy.
