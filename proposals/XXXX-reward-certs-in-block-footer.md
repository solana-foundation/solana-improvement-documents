---
simd: '0402'
title: Finalization Certificate in Block Footer
authors:
  - Quentin Kniep (Anza)
  - ksn6 (Anza)
  - Wen Xu (Anza)
category: Standard
type: Core
status: Review
created: 2025-11-11
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

The [Rewards](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0326-alpenglow.md#rewards) section of the Alpenglow consensus protocol (SIMD-0326) describes how validators are rewarded for voting.


We propose augmenting `BlockMarkerV1`, introduced in SIMD-0307, to allow slot leaders to insert optional skip and notarization reward certificates.

This will enable validators observing the block marker to pay the validators (and delegators) rewards for voting as per the Alpenglow consensus protocol.


## Motivation

Alpenglow introduces a new way of voting (validators no longer send vote transactions), instead they broadcast their votes to all other validators.
Since, votes transactions are no longer available no chain, Alpenglow also introduces a new way for validators to get rewarded for voting.
In slot *s+8*, the corresponding leader can post up to two vote aggregates (a notarization aggregate and/or skip aggregate) for all the votes it saw for slot *s*.

The block footer mechanism, introduced in SIMD-0307, serves as an ideal location to place these aggregates since each valid block must contain a footer and the footer is designed to disemminate per block information to validators on the network.

## Dependencies

- Alpenglow is specified in [SIMD 326](https://github.com/solana-foundation/solana-improvement-documents/pull/326)

- Block footer is specified in [SIMD 307](https://github.com/solana-foundation/solana-improvement-documents/pull/307)

## New Terminology

- **Skip Reward Certificate**: this is an aggregate of all the skip votes that were recorded by the leader.

- **Notarization Reward Certificate**: this is an aggregate of all the notarization otes that were recorded by the leader.

## Detailed Design

### Data Layout in Block Footer

#### Finalization Certificate for Observability data structure

```rust
struct SkipRewardCertificate {
    slot: Slot,
    signature: BLSSignatureCompressed,
    bitmap: Vec<u8>,
    
}

struct NotarRewardCertificate {
    slot: Slot,
    block_id: Hash,
    signature: BLSSignatureCompressed,
    bitmap: Vec<u8>,
    
}
```

Where `Slot` is u64, `Hash` is [u8; 32], and `BLSSignatureCompressed` is specified in `bls-signatures` is a 96 bytes array.

`solana-signer-store` describes the bitmap format.
Both aggregates use the base2-encoding.

Block Footer Extension

```rust
struct BlockFooterV1 {
    bank_hash: Hash,                                                // Introduced in V1 (SIMD-0298)
    block_producer_time_nanos: u64,                                 // Introduced in V1 (SIMD-0307)
    block_user_agent: Vec<u8>,                                      // Introduced in V1 (SIMD-0307)
    skip_reward_certificate: Option<SkipRewardCertificate>,         // New in this SIMD
    notar_reward_certificate: Option<NotarRewardCertificate>,       // New in this SIMD
}
```

**Note on Versioning and Field Ordering**: While adding fields to the footer would typically warrant a version increment, we maintain `footer_version=1` for simplicity.
We only make these atypical changes in light of the fact that, as of December 2025, clients do not yet disseminate block footers or block markers, making this an appropriate time to modify the version 1 format before widespread adoption.

#### Serialization Format

The extended block footer serializes within a `BlockComponent` as follows:

```
+---------------------------------------+
| Entry Count = 0             (8 bytes) |
+---------------------------------------+
| Marker Version = 1          (2 bytes) |
+---------------------------------------+
| Variant ID = 0              (1 byte)  |
+---------------------------------------+
| Length                      (2 bytes) |
+---------------------------------------+
| Version = 1                 (1 byte)  |
+---------------------------------------+
| bank_hash                  (32 bytes) |
+---------------------------------------+
| block_producer_time_nanos   (8 bytes) |
+---------------------------------------+
| block_user_agent_len         (1 byte) |
+---------------------------------------+
| block_user_agent        (0-255 bytes) |
+---------------------------------------+
| skip_reward_present          (1 byte) | ← NEW
+---------------------------------------+
| skip_reward_cert           (variable) | ← NEW
+---------------------------------------+
| notar_reward_present         (1 byte) | ← NEW
+---------------------------------------+
| notar_reward_cert          (variable) | ← NEW
+---------------------------------------+
```

If `skip_reward_present` is 0, then there is no `skip_reward`.
Likewise for `notar_reward`.


#### Skip reward certificate Serialization

If `skip_reward_cert` is present, it is serialized as follows:

```
+---------------------------------------+
| slot                        (8 bytes) |
+---------------------------------------+
| signature                  (96 bytes) |
+---------------------------------------+
| bitmap_len                  (2 bytes) |
+---------------------------------------+
| bitmap                     (variable) |
+---------------------------------------+
```

#### Notarization reward certificate Serialization

If `notar_reward_cert` is present, it is serialized as follows:

```
+---------------------------------------+
| slot                        (8 bytes) |
+---------------------------------------+
| block_id                   (32 bytes) |
+---------------------------------------+
| signature                  (96 bytes) |
+---------------------------------------+
| bitmap_len                  (2 bytes) |
+---------------------------------------+
| bitmap                     (variable) |
+---------------------------------------+
```


### Field Population by leader

As described above, while producing a block at slot `s`, the leader should include aggregates of all the notarization and skip votes it observed for slot `s-8`.

Note that notarization votes include a block id and different honest validators may vote notarization for different block ids.
In order to validate the notarization reward certificate, the block id has to be included in the certificate and the leader is allowed to submit only 1 notarization reward certificate.
This means that if a leader has received notarization votes for different block ids, it can only submit votes for one block id.
As described in the Alpenglow SIMD, the leader's reward is a function of how many votes it includes in the certificate so it is incentivized to select the block id that received the most votes.

### Field Validation by non-leaders

Validators MUST enforce the following rules:

1. BLS Validity: All certificates provided must pass BLS signature verification.

Any violation should cause the block to be invalidated and the remainder of the leader window should be skipped.

## Alternatives Considered

**Use a single base-3 encoded Certificate**: Instead of submitting two separate base-2 encoded certificates, the leader could submit a single base-3 encoded certificate similar to the `NotarizeFallback` certificate in Alpenglow.
This option offers some space savings at the cost of more complex processing and code.
This option was rejected because in the normal case, we expect that most validators would be casting notarization votes and the space savings was deemed to not be significant enough to warrant the additional complexity.

## Impact

Invalid certificates will cause the block to be marked invalid.

## Security Considerations

N/A

## Backwards Compatibility

Not backward compatible.
