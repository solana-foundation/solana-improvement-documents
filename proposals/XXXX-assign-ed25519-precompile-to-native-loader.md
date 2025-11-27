---
simd: 'XXXX'
title: Assign Ed25519 Precompile to Native Loader
authors:
  - Dean Little (Blueshift)
  - David Leung (Blueshift)
category: Standard
type: Core
status: Review
created: 2027-11-27
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal restores nominal functionality of the Ed25519 precompile to 
testnet witohut any changes to mainnet and devnet.

## Motivation

As a result of the Ed25519 program belonging to the System Program instead of 
the Native Loader on testnet due to an old cluster configuration issue 
https://github.com/solana-labs/solana/pull/23219, recent activation of SIMD-0186
broke the Ed25519 precompile on the testnet cluster due to this branch of code 
*correctly* enforcing the behavior we should have initially solved for: 

https://github.com/anza-xyz/agave/blob/89ead14a0fd576a13c37513419f8e2406769684f/svm/src/account_loader.rs#L607-L610

## New Terminology

N/A

## Detailed Design

Introduce a feature gated change to `bank.rs` that, upon activation, updates 
the owner of the Ed25519 program to the Native Loader program if it is not 
already owned by it, also explicitly flagging its account as executable, 
leaving all other fields unchanged.

```rust
if new_feature_activations
    .contains(&feature_set::assign_ed25519_precompile_to_native_loader::id())
{
    if let Some(account) = self
        .get_account_with_fixed_root(&solana_sdk_ids::ed25519_program::id())
        .and_then(|account| {
            if !native_loader::check_id(account.owner()) {
                Some(account)
            } else {
                None
            }
        })
    {
        let new_account = AccountSharedData::from(Account {
            owner: native_loader::ID,
            executable: true,
            ..Account::from(account)
        });

        self.store_account(&solana_sdk_ids::ed25519_program::id(), &new_account);
    }
}
```

## Alternatives Considered

- Patch SIMD-0186 to add an exception in the case of the Ed25519 program
- Leave the functionality broken on testnet
- Only activate feature on testnet

## Impact

When activated on testnet, this feature will restore nominal behavior to the 
Ed25519 precompile. When activated on mainnet and devnet, nothing will change.

## Security Considerations

None.

## Drawbacks

None.

## Backwards Compatibility

This feature is backwards compatible with mainnet and devnet, but results in 
breaking changes on testnet required to resolve the existing regression.
