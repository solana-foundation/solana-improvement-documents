---
simd: '0023'
title: Consensus and Transaction Inclusion Verification
authors:
  - Harsh Patel (Tinydancer)
  - Anoushk Kharangate (Tinydancer)
  - x19 (Tinydancer)
category: Standard
type: Core
status: Draft
created:  2023-05-25
---

## Summary

Add a new RPC method that returns a proof that a transaction (tx) is included in a specific slot. 

With this new RPC method, users can verify that supermajority has voted on the slot that their transaction was included in without fully trusting the RPC provider. This is the first step in implementing a consenus verifying client as first described in [SIMD](https://github.com/solana-foundation/solana-improvement-documents/pull/10).

## Motivation

Currently, for a user to validate whether their transaction was included in a block, it needs to trust the confirmation from an RPC provider. This is a glaring attack vector for malicious RPC actors that could lie to users if it's in their own interest. 

To combat this, a user must run a full node to process the entire ledger and verify the blocks and votes themselves. The downside of this is that its very costly to run a full node which makes inaccessible to everyday users. 

One solution to this problem is to use diet clients to verify transaction confirmations without fully trusting the RPC. This SIMD is the first step towards implementing diet clients for Solana and provides a way for users to verify their transaction is included in a block and that the block has recieved a supermajority of votes.

## Alternatives Considered

None

## New Terminology

`TransactionProof`: A structure containing necessary information to verify if a transaction was included in the bankhash of a slot.

#### New RPC Methods
The new RPC method would be called `get_transaction_proof` which would take a slot number as input and return a `TransactionProof` struct
```rs
// new RPC method 
pub async fn get_transaction_proof(&self, slot: Slot) -> Result<TransactionProof>;

// new RPC struct to verify tx inclusion
pub struct TransactionProof {
  pub entries: Vec<Entry>,
  pub start_blockhash: Hash,
  pub parent_hash: Hash, 
  pub accounts_delta_hash: Hash, 
  pub signature_count_buf: [u8; 8],
}
```
All the variables are accessible from the rpc's blockstore and bank_forks variables, so no changes to the rest of the codebase will be required. The following is psuedocode of the RPC method:
```rs
pub async fn get_transaction_proof(&self, slot: Slot) -> Result<TransactionProof> {
  // first retrieve all the entries 
  let (entries, _, is_full) = self.blockstore.get_slot_entries_with_shred_info(slot, 0, false)
  // require all of the entries 
  assert!(is_full)

  let bank_forks = self.bank_forks.read().unwrap();
  let bank = bank_forks.get(slot);

  // get variables used to compute bank hash 
  let parent_hash = bank.parent_hash();
  let accounts_delta_hash = bank
      .rc
      .accounts
      .accounts_db
      .calculate_accounts_delta_hash(slot).0;
  let mut signature_count_buf = [0u8; 8];
  LittleEndian::write_u64(&mut signature_count_buf[..], bank.signature_count());

  // get the start_hash for the slot (will be the last entry's hash from slot-1)
  let start_blockhash = self.blockstore.get_slot_entries_with_shred_info(slot-1, 0, false).last().hash;

  Ok(TransactionProof{ 
      entries,
      start_blockhash,
      parent_hash, 
      accounts_delta_hash, 
      signature_count_buf,
  })
}
```
Using the `get_transaction_proof` RPC call, a client can verify a transaction with the following steps:
- first, for a given transaction signature, find the slot which its included in
- call the `get_transaction_proof` RPC method with that slot as input
- verify the `entries` are valid PoH hashes starting with the hash `start_blockhash`
- verify that the transaction signature is included in one of the `entries`
- reconstruct the expected bankhash using the other variables in the struct and the final entry's hash

Below is client pseudocode for verifying a transaction:
```rust 
let tx_sig = "..."; // tx signature of interest
let slot = 19; // slot which includes the tx

// call the new RPC
let tx_proof: TransactionProof = get_tx_proof(slot, endpoint);

// verify the entires are valid
let verified = tx_proof.entries.verify(&tx_proof.start_blockhash);
assert!(verified);

// verify that the transaction signature is included in one of the `entries`
let mut start_hash = &tx_proof.start_blockhash;
let mut was_verified = false; 
for entry in entries.iter() {
    // find Entry which includes tx sig
    let tx_is_in = entry.transactions.iter().any(|tx| { 
        tx.signatures.contains(&tx_sig)
    });
    if tx_is_in { 
      // verify Entry includes tx 
      let hash = next_hash(start_hash, entry.num_hashes, &entry.transactions);
      assert!(hash == entry.hash);
      was_verified = true;
      break;
    }
    start_hash = &entry.hash;
}
assert!(was_verified);

// recompute the bank hash 
let last_blockhash = entries.last().unwrap().hash;
let bankhash = hashv(&[
    tx_proof.parent_hash.as_ref(),
    tx_proof.accounts_delta_hash.as_ref(),
    tx_proof.signature_count_buf.as_ref(), 
    last_blockhash.as_ref()
]);
```
Once we computed the expected bankhash, we can parse vote transactions which vote on that bankhash, and assert that a supermajority (>= 2/3 of stake) has voted on it.

## Impact

This proposal will improve the overall security and decentralisation of the Solana network allowing users to access the blockchain in a trust
minimised way unlike traditionally where users had to fully trust their RPC providers. Dapp developers don't have to make any changes as wallets can easily integrate the client making it compatible with any dapp.

## Security Considerations

### Trust Assumptions

While this SIMD greatly increases the user's trustlessness required to an RPC, the light client will still needs to make certain trust assumptions. This includes finding a trusted source for the validator set per epoch (including their pubkeys and stake weights) and trusting that all transactions are valid (incase the supermajority is corrupt). We plan to solve these problems in future SIMDs to provide a full trustless setup including data availability sampling and fraud proving which will only require a single honest full node.

## Future Work

While there are improvements and additional features that can be made (as mentioned in [this proposal](https://docs.solana.com/proposals/simple-payment-and-state-verification)), including using merkle trees to compute the blockhash (instead of the current sequential implementation), transaction status codes, and validator set verification, we chose to keep this SIMD self-contained and only add a new RPC method with no changes to the protocol. Optimizations will be left to future SIMDs which will build off of this one.
