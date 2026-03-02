---
simd: '0464'
title: Vote Account Initialize V2
authors:
  - Wen Xu (Anza)
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Review
created: 2026-02-05
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This proposal introduces the `InitializeAccountV2` instruction to the Vote
program, which allows creating new vote accounts with all vote state v4 fields
— including BLS public keys — at account creation time.

## Motivation

The existing `InitializeAccount` instruction for the Vote program does not
support setting all vote state v4 fields at creation time. After the activation
of vote state v4 ([SIMD-0185]), users must create an account with
`InitializeAccount` and then use multiple separate instructions to configure
fields like commission rates, collector accounts, and BLS public keys.

`InitializeAccountV2` provides a unified way to set all vote state v4 fields
during account creation, streamlining the process for validators setting up
new vote accounts.

## New Terminology

N/A

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0387]: BLS Pubkey management in vote account**

    Specifies BLS public key generation and proof of possession verification

### Feature Activation Ordering

The `InitializeAccountV2` instruction introduced by this proposal allows
setting all vote state v4 fields at account creation time. To prevent early
access to these fields before their respective feature gates are activated,
**this SIMD's feature gate MUST be activated after all of the following**:

- **[SIMD-0180]: Use Vote Account Address To Key Leader Schedule**

    Prerequisite for vote state v4 features to be operational

- **[SIMD-0185]: Vote Account v4**

    Adds the vote state v4 structure including the BLS public key field

- **[SIMD-0291]: Commission Rate in Basis Points**

    Enables the `inflation_rewards_commission_bps` field via `UpdateCommissionBps`

- **[SIMD-0232]: Custom Commission Collector Account**

    Enables the `inflation_rewards_collector` and `block_revenue_collector`
    fields via `UpdateCommissionCollector`

- **[SIMD-0123]: Block Revenue Distribution**

    Enables the `block_revenue_commission_bps` field via `UpdateCommissionBps`

This ordering ensures that each vote state v4 field can only be set through
its designated instruction until all features are active, at which point
`InitializeAccountV2` provides a unified way to set all fields at account
creation.

[SIMD-0123]: https://github.com/solana-foundation/solana-improvement-documents/pull/123
[SIMD-0180]: https://github.com/solana-foundation/solana-improvement-documents/pull/180
[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/pull/185
[SIMD-0232]: https://github.com/solana-foundation/solana-improvement-documents/pull/232
[SIMD-0291]: https://github.com/solana-foundation/solana-improvement-documents/pull/291
[SIMD-0387]: https://github.com/solana-foundation/solana-improvement-documents/pull/387

## Detailed Design

### Add InitializeAccountV2

A new instruction for initializing a vote account with all vote state v4 fields
will be added to the vote program with the enum discriminant value of `16u32`
little endian encoded in the first 4 bytes.

```rust
pub enum VoteInstruction {
    /// # Account references
    ///   0. `[WRITE]` Uninitialized vote account
    ///   1. `[SIGNER]` New validator identity (node_pubkey)
    ///   2. `[WRITE]` Inflation rewards collector (or vote account if same)
    ///   3. `[WRITE]` Block revenue collector (or vote account if same)
    InitializeAccountV2(VoteInitV2), // 16u32
}
```

```rust
pub const BLS_PUBLIC_KEY_COMPRESSED_SIZE: usize = 48;
pub const BLS_PROOF_OF_POSSESSION_COMPRESSED_SIZE: usize = 96;

pub struct VoteInitV2 {
  pub node_pubkey: Pubkey,
  pub authorized_voter: Pubkey,
  pub authorized_voter_bls_pubkey: [u8; BLS_PUBLIC_KEY_COMPRESSED_SIZE],
  pub authorized_voter_bls_proof_of_possession: [u8; BLS_PROOF_OF_POSSESSION_COMPRESSED_SIZE],
  pub authorized_withdrawer: Pubkey,
  pub inflation_rewards_commission_bps: u16,
  pub block_revenue_commission_bps: u16,
}
```

Upon receiving the transaction, the vote program will perform a BLS
verification on the submitted BLS public key and associated proof of
possession, as described in [SIMD-0387]. The transaction will fail if the
verification fails.

For each collector account (indices `2` and `3`), the vote program MUST perform
the following validation, matching the checks required by
`UpdateCommissionCollector` in [SIMD-0232]:

1. If the collector is not equal to the vote account, it must be system program
   owned. Otherwise return `InstructionError::InvalidAccountOwner`.
2. The collector must be rent-exempt. Otherwise return
   `InstructionError::InsufficientFunds`.
3. The collector must be writable (not a reserved account). Otherwise return
   `InstructionError::InvalidArgument`.

If all checks pass, the new vote account is created with the given parameters.
The inflation rewards collector address is taken from account index `2` and the
block revenue collector address is taken from account index `3`.

The BLS PoP verification will cost 34,500 CUs, as described in [SIMD-0387].

## Impact

### Before feature gate in this SIMD is activated

There is no change. Users continue to create vote accounts using the legacy
`InitializeAccount` instruction, then set newer fields using their respective
instructions (ie. `UpdateCommissionBps`).

### After the feature gate in this SIMD is activated

New vote accounts can be created using `InitializeAccountV2`, which sets all
vote state v4 fields at creation time including the BLS public key and proof of
possession. The legacy `InitializeAccount` instruction remains available.

## Security Considerations

BLS public key verification and proof of possession are specified in
[SIMD-0387]. The same security considerations for BLS rogue-key attacks and
replay attacks described there apply to `InitializeAccountV2`.

## Alternatives Considered

N/A
