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

This SIMD describes the changes required to allow users to
verify that a supermajority of the stake has voted on the slot that their
transaction was included in the block without fully trusting the RPC provider.

The main change includes:

- Modifying the Bankhash to add a transaction receipt root of the transaction
  receipt merkle tree. This root would be a 32 byte SHA-256 hash.

## Motivation

Currently, for a user to validate whether their transaction is valid and included
in a block it needs to trust the confirmation from the RPC. This has been a glaring
attack vector for malicious actors that could lie to users if it's in their own interest.

To combat this, mature entities like exchanges run full nodes that process the
entire ledger and can verify entire blocks. The downside of this is that it's
very costly to run a full node which makes it inaccessible to everyday users,
exposing users to potential attacks from malicious nodes.
This is where diet clients come in, users run the client to verify
the confirmation of their transaction without trusting the RPC.

## Alternatives Considered

None

## New Terminology

Receipt: A structure containing a 64 byte version, a transaction message hash 
and its execution status.

Receipt Root: The root hash of a binary merkle tree of transaction receipts.

## Detailed Design

### Modifying the Bankhash

We propose the following change:

Using the receipt data structure and the receipt merkle tree which is formally
defined in this [SIMD]([https://github.com/tinydancer-io/solana-improvement-documents](https://github.com/tinydancer-io/solana-improvement-documents/blob/transaction-receipt/proposals/0064-transaction-receipt.md))

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
