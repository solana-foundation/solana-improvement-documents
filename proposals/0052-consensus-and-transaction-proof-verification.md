---
simd: '0052'
title: Consensus and Transaction Proof Verification
authors:
  - Harsh Patel (Tinydancer)
  - Anoushk Kharangate (Tinydancer)
  - x19 (Tinydancer)
category: Standard
type: Core
status: Draft
created: 2023-05-30
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This SIMD describes the overall design and changes required that allows users to
verify that a supermajority of the validators has voted on the slot that their
transaction was included in the block without fully trusting the RPC provider.

This includes two main changes:

1) Adding a new RPC method which provides a proof that a transaction has been
   included in a slot
2) Modifying the blockhash to be computed as a Merkle Tree and
   includes transaction statuses

This SIMD is the first step in implementing a consensus verifying client as first
described in [SIMD #10](https://github.com/solana-foundation/solana-improvement-documents/pull/10)
and a majority of the changes mentioned in
the accepted [Simple Payment and State Verification Proposal](https://docs.solana.com/proposals/simple-payment-and-state-verification).

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

However, this is only the consensus verifying stage of the client, and with only
these changes, the RPC provider can still trick users, hence we also discuss future
work that will be implemented in futureSIMDs to provide a fully trustless setup.

## Alternatives Considered

None

## New Terminology

TransactionProof: A structure containing necessary information to verify if a
transaction was included in the bank hash of a slot.

```rs
// new RPC struct to verify tx inclusion
pub struct TransactionProof {
  pub proof: Vec<Hash>,
  pub parent_hash: Hash,
  pub accounts_delta_hash: Hash,
  pub signature_count_buf: [u8; 8],
}
```

The proof variable will provide a Merkle hashpath from the transaction to the blockhash;
which is then used with the other variables to compute the bankhash.

## Detailed Design

The protocol interaction will be as follows:

- A user sends a transaction and it lands in slot N
- The user requests proof that the transaction is included in slot N using
  the new RPC method
- The user verifies that the proof includes the transaction signature
  and a success status
- The user constructs the merkle tree to derive the root hash
- The user computes the expected bankhash using the root hash, `parent_hash`,
  `accounts_delta_hash` and `signature_count`
- The user retrieves the epoch’s current validator set and stake amounts from
  a trusted source (making this step trustless is future work)
- The user requests blocks from slots > N (up to 32 slots past N given the
  32 block depth finality)
- The user parses vote transactions from the blocks, verifying their signature,
  and computing the sum of stake which voted on
  the bankhash they computed in step 3
- If the sum of stake is greater than or equal to 2/3 of the total stake then
  their transaction has been finalized under a supermajority trust assumption
  (making this assumption require only a single honest validator is also future work)

### Modifying the Blockhash

We propose modifying the blockhash computation to:

1) Compute the blockhash using a Merkle Tree of entries
2) Include the status (either succeeding or failing) of each transaction
   in each Entry leaf

To produce the blockhash for a slot, the current implementation hashes Entries in
a sequential way which requires a O(N) proof size to provide a hashpath from a
transaction to a blockhash. Implementing change 1) would allow for a more efficient
O(log(N)) proof size. The current Entry implementation already hashes transaction
signatures using a Merkle Tree to get the Entry’s hash, so this change would only
modify how the Entry’s hashes are hashed together to get a blockhash.

For change 2), the blockhash is currently computed using only transaction signatures,
and does not include transaction statuses which means we are unable to prove if
a transaction has succeeded or failed. Implementing 2) would enable verifying
a transactions status, as mentioned in the [accepted proposal](https://docs.solana.com/proposals/simple-payment-and-state-verification#transaction-merkle)
and in a [previous github issue](https://github.com/solana-labs/solana/issues/7053)).

Fig #1 shows an example hashpath from a transaction and its signature to a
bankhash with both of the proposed changes implemented.
![Fig #1](https://github.com/tinydancer-io/solana-improvement-documents/assets/32778608/5370950d-e27b-4c1b-9f04-6e9164789e65)

#### New RPC Methods

We also need a new RPC method to provide proofs to clients. This method would be
called `get_transaction_proof` which would take a transaction signature as input
and return a `TransactionProof` struct

```rs
// new RPC method
pub async fn get_transaction_proof(&self, signature: Signature) -> Result<TransactionProof>;
```

Below is psuedocode of the RPC method:

```rs
async fn get_transaction_proof(&self, s: Signature) -> Result<TransactionProof> {
  // first retrieve all the entries
  let slot = self.get_slot_of_signature(&s);
  let (entries, _, is_full) = 
    self.blockstore.get_slot_entries_with_shred_info(slot, 0, false)  
  // require all of the entries
  assert!(is_full)

  // compute the Merkle hashpath from the signature and status to the blockhash 
  let proof = entries.get_merkle_proof(&s);

  // get variables used to compute the bankhash
  let bank_forks = self.bank_forks.read().unwrap();
  let bank = bank_forks.get(slot);

  let parent_hash = bank.parent_hash();
  let accounts_delta_hash = bank
      .rc
      .accounts
      .accounts_db
      .calculate_accounts_delta_hash(slot).0;
  let mut signature_count_buf = [0u8; 8];
  LittleEndian::write_u64(&mut signature_count_buf[..], bank.signature_count());

  Ok(TransactionProof{
      proof,
      parent_hash,
      accounts_delta_hash,
      signature_count_buf,
  })
}
```

Below is client pseudocode for verifying a transaction:

```rust
let tx_sig = "..."; // tx signature of interest
let slot = 19; // slot which includes the tx

// call the new RPC
let tx_proof: TransactionProof = get_tx_proof(&tx_sig, endpoint);

// verify the transaction signature proof is valid and status is success
let leaf = hash_leaf!([tx_sig, TxStatus::Success]);
let verified = tx_proof.proof.verify_path(&leaf);
assert!(verified);

// compute the blockhash
let last_blockhash = tx_proof.proof.get_root();

// compute the expected bankhash
let bankhash = hashv(&[
    tx_proof.parent_hash.as_ref(),
    tx_proof.accounts_delta_hash.as_ref(),
    tx_proof.signature_count_buf.as_ref(),
    last_blockhash.as_ref()
]);

// parse vote transactions and stake amounts on expected bankhash
let (voted_stake_amount, total_stake_amount) = parse_votes_from_blocks(slot, bankhash)

// validate supermajority voted for expected bankhash
let supermajority_verified = 3 * voted_stake_amount >= 2 * total_stake_amount;
assert!(supermajority_verified)
```

## Impact

This proposal will improve the overall security and decentralization of the Solana
network allowing users to access the blockchain in a trust minimized way unlike
traditionally where users had to fully trust their RPC providers. Dapp developers
don't have to make any changes as wallets can easily integrate the client making
it compatible with any dapp.

## Security Considerations

### Trust Assumptions and Future Work

While this SIMD greatly reduces the user's trust in an RPC, the light client will
 still need to make certain trust assumptions. This includes finding a trusted
 source for the validator set per epoch (including their pubkeys and stake weights)
 and trusting that all transactions are valid (in case the supermajority is corrupt).
 We plan to solve these problems in future SIMDs to provide a full trustless setup
 including data availability sampling and fraud proving which will only require a
 single honest full node.

## Backwards Compatibility *(Optional)*
