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
Another solution is to use the gossip plane to read votes out of the CRDS optimistically and then confirm by verifying inclusion with the blockhash. The advantage being that no changes are required to the validator to read the votes. However this has a couple of drawbacks:
- Votes have a high probability of getting dropped after being inserted into the CRDS as sometimes validators can vote on older slots after voting on the newer slot.
- Validators aren't obligated to propagate gossip changes and have reason to not do so as it reduces egress costs.

We could also just directly make these calls from the client itself but its much faster and more convenient to do it on server side.


## New Terminology

TransactionProof: A structure containing necessary information to verify if a transaction was included in the bank hash of a slot.

## Detailed Design
The protocol interaction will be as follows:
1. User makes a transaction using their wallet in slot N.
2. The client then reads the validator set from gossip and requests vote signatures and stake commitments for slot N to (N+32) given the 32 block depth finality.  
3. It first validates whether the validator identities match the set from gossip.
4. Then proceeds to validate the vote signatures.
5. The client also has to sync the epoch stake history from genesis from the entrypoint light clients eventually can be requested from multiple other light clients).
6. Next it checks if the stake weights returned are valid with the local stake history that wasy synced from entrypoint and if the stake is >= 67% of the total stake.
7. Next it will request the transaction proof using the slot and transaction signature. It will then perform the following checks:
  -  Verify the entries by checking that hashing the start hash of the slot `num_hash` times results in the same hash as the entry.
  -  Check if the transaction signature is included in any of the slots entries.
  -  Check if transaction is included in the poh hash by calculating `hash(prev_hash,hash(transaction_signatures))` and if it matches the entry hash.
8. Reconstruct the bank hash with blockhash(entry hash), parent_hash, accounts_delta_hash and signature_count and check if all votes voted on this bank hash by parsing them.
9. If all these checks are valid the slot can be marked as confirmed under a supermajority trust assumption.

#### Types of Light Clients
1. **Entrypoint / Hub Clients** - These will be holding the snapshot with the stake history from genesis to current slot and be periodically syncing with the latest epoch.
   
3. **Ultra Light Clients** - These will be clients running in wallets on mobile or browser that will TOFU(Trust On First Use) the stake from the entrypoint clients and verify if the user transactions are voted on by the supermajority.

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
The light client makes certain trust assumptions that reduce reliance on RPC but these assumptions have tradeoffs. The ultra light clients verify if the supermajority has voted on the block, this is could be an issue if the supermajority itself is corrupt and is trying to vote on an invalid transaction.

## Future Work
The future iterations of the light client that include data availability sampling and fraud proving should address this issue reduce the trust assumption to single honest node. We also rely on a entrypoint light client that has the snapshot and serves the stake history which is also a point of trust.

Additionally, while there are improvements and additional features that can be made (as mentioned in [this proposal](https://docs.solana.com/proposals/simple-payment-and-state-verification)), including using merkle trees to compute the blockhash (instead of the current sequential implementation), transaction status codes, and validator set verification, we chose to keep this SIMD self-contained and only add a new RPC method with no changes to the protocol. Optimizations will be left to future SIMDs which will build off of this one.

