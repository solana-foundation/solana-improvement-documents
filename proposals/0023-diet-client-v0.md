---
simd: '0023'
title: Diet Client V0(Consensus Client)
authors:
  - Harsh Patel (Tinydancer)
  - Anoushk Kharangate (Tinydancer)
category: Standard
type: Core
status: Draft
created:  2023-04-26
---

## Summary
Add a new RPC call and `getBlockHeaders()` to add support for consenus verifying clients as first described in this [SIMD](https://github.com/solana-foundation/solana-improvement-documents/pull/10)

## Motivation
For a user to validate whether their transaction is valid and included in a block it needs to trust the confirmation from the RPC. This has been a glaring attack vector for malicious actors that could lie to users if it's in their own interest. To combat this mature entities like exchanges run full nodes that process the entire ledger and can verify entire blocks. The downside of that being very high cost to run a full node making it less accessible to everyday users, in effect exposing users to potential attacks from mailicious nodes. 

This is where diet clients come in, users run the client to verify confirmation of their transaction without trusting the RPC. The SIMD is the first step towards implementing the diet client by proposing a small change to the rpc service that allows the client to validate if supermajority stake actually signed off on a block. 

This ensures that at-least the user doesn't have to trust the RPC service that is centralised and can rather trust the supermajority of the network which is less propable to be corrupt than a malicious RPC. However it is not impossible, hence the full diet clietn implementation discusses further steps to counter that and this is only the consensus verifying stage of the client.


## Alternatives Considered
Another solution is to use the gossip plane to read votes out of the CRDS optimistically and then confirm by verifying inclusion with the blockhash. The advantage being that no changes are required to the validator to read the votes. However this has a couple of drawbacks:
- Votes have a high probability of getting dropped after being inserted into the CRDS as sometimes validators can vote on older slots after voting on the newer slot.
- Validators aren't obligated to propagate gossip changes and have reason to do so as it reduces egress costs.


## New Terminology

BlockHeader: A structure containing all vote identities, vote signatures and stake amounts that has voted on a block.

## Detailed Design

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
