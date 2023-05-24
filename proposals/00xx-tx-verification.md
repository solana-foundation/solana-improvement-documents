---
simd: '00XX'
title: Transaction Verification RPC Method
authors:
  - Harsh Patel (Tinydancer)
  - Anoushk Kharangate (Tinydancer)
  - x19 (Tinydancer)
category: Standard
type: Core
status: Draft
created:  2023-05-24
---

## Summary

Add a new RPC method that returns data to prove a transaction (tx) is included in a specific slot. 

## Motivation

The first step for Solana light/diet clients is to verify a transaction was processed in a specific slot. This requires a hash path from a transaction's signature to a slot's bankhash as proof (as noted in a [previous diet client SIMD](https://github.com/solana-foundation/solana-improvement-documents/pull/10)). Since validators sign votes on bankhashes, once we have this proof, we can also verify a transaction has received a supermajority of votes. Current RPC methods don't provide enough information so we need a new RPC method for this.

## Alternatives Considered

None

## New Terminology

None

## Detailed Design

The new RPC method would be called `get_transaction_proof` which would take a slot number as input and return a `TransactionProof` struct: 

```rust 
// new RPC method 
pub async fn get_transaction_proof(&self, slot: Slot) -> Result<TransactionProof>;

// new RPC struct to verify tx inclusion
pub struct TransactionProof {
  // used to verify hash([start_blockhash, entires]) = bank_hash
  // also verify the tx_signature of interest is included in an Entry
  pub entries: Vec<Entry>,
  pub start_blockhash: Hash,
  // used to reconstruct `bank_hash`
  pub parent_hash: Hash, 
  pub accounts_delta_hash: Hash, 
  pub signature_count_buf: [u8; 8],
}
```

All the variables are accessible from the rpc's blockstore and bank_forks variables, so no changes to the rest of the codebase will be required. The following is psuedocode of the RPC method:  

```rust
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

## Future Work 

While there are improvements and additional features that can be made (as mentioned in [this proposal](https://docs.solana.com/proposals/simple-payment-and-state-verification)), including using merkle trees to compute the blockhash (instead of the current sequential implementation), transaction status codes, and validator set verification, we chose to keep this SIMD self-contained and only add a new RPC method with no changes to the protocol. Optimizations will be left to future SIMDs which will build off of this one.

## Impact

This proposal will improve the overall security and decentralisation of the Solana network allowing users to verify their transactions in a trustless way, unlike traditionally where users had to fully trust their RPC providers. Since its only a new RPC method, Dapp developers don't have to make any special changes, as wallets and can easily integrate the verification to be compatible with any transaction.

## Security Considerations

While this SIMD greatly increases the trustlessness on a single RPC, the light client will still need to make certain trust assumptions. This includes finding a trusted source for the validator set per epoch (including their pubkeys and stake weights) and trusting that all transactions are valid. We plan to solve these problems in future SIMDs to provide a full trustless setup including data availability sampling and fraud proving.
