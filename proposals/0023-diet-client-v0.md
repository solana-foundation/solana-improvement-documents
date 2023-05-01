---
simd: '0023'
title: Diet client v0
authors:
  - Harsh Patel (Tinydancer)
  - Anoushk Kharangate (Tinydancer)
category: Standard
type: Core
status: Draft
created:  2023-04-26
---

## Summary
Add two new RPC calls `getShreds()` and `getBlockHeaders()` to add support for diet clients as first described in this [SIMD](https://github.com/solana-foundation/solana-improvement-documents/pull/10)

## Motivation
For light clients to be possible on Solana, there is a need to access ledger data in the form of shreds for Data Availability 
sampling to verify ledger data. In addition to verifying the ledger, we also need to verify the consensus for a particular block.
Solana doesn't have the concept of blockheaders like Ethereum, so we added a new data structure in the solana-transaction-status 
crate to address this issue.

## Alternatives Considered
Another solution is to use the gossip plane to read votes out of the CRDS optimistically and then confirm by verifying inclusion with the blockhash. The advantage being that no changes are required to the validator to read the votes. However this has a couple of drawbacks:
- Votes have a high probability of getting dropped after being inserted into the CRDS as sometimes validators can vote on older slots after voting on the newer slot.
- Validators aren't obligated to propagate gossip changes and have reason to do so as it reduces egress costs.


## New Terminology

BlockHeader

## Detailed Design

We introduce two RPC calls:
- #### getShreds
  ```
  pub async fn get_shreds(
          &self,
          slot: Slot,
          shred_indices: Vec<u64>,
          config: Option<RpcShredConfig>,
      ) -> Result<GetShredResponse>
  ```
This call would allow for data availability sampling, this is specifically added to the `rpc.rs` file as an additional method to
`JsonRpcRequestProcessor` where we pass in the slot, the indices of the required shreds and the config which contains the
CommitmentConfig of the block. Apart from this we also made the shred data structures accessible to other crates from
`pub(crate)` => `pub`.

- #### getBlockHeaders
```
pub async fn get_block_headers(
        &self,
        slot: Slot,
        config: Option<RpcEncodingConfigWrapper<RpcBlockConfig>>,
    ) -> Result<BlockHeader> 
```

```
pub struct BlockHeader {
    pub vote_signature: Vec<Option<String>>,
    pub validator_identity: Vec<Option<Pubkey>>,
    pub validator_stake: Vec<Option<u64>>,
}
```
This function will return a BlockHeader, a data structure storing a list of:
 - Signatures of validators who voted on that block
 - The public keys or 'identities' of the validators who voted on that block.
 - The stake amounts of each of those validators.


## Impact

How will the implemented proposal impacts dapp developers, validators, and core contributors?

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks *(Optional)*

Why should we not do this?
