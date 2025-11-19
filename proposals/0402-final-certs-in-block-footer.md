---
simd: '0402'
title: Finalization Certificate in Block Footer
authors:
  - To be filled
category: Standard
type: Core
status: Review
created: 2025-11-11
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD proposes adding an Alpenglow finalization certificate to the
Block Footer for enhanced observability. This way anyone who only observes
blocks can understand that the blocks are finalized without knowing the details
of all-to-all communication between the validators.

## Motivation

Before Alpenglow, validator votes were expressed as on-chain transactions that
updated vote accounts, each functioning as a state machine tracking the voting
activity of a staked validator. Alpenglow (SIMD-0326) moves away from this
model by having validators send vote messages directly to one another,
improving the speed and efficiency of consensus.

This shift removes the on-chain visibility previously provided by vote
transactions and vote account state, which zero-staked validators currently
rely on to infer validator delinquency. It also affects any party that depends
on the ability to observe votes on-chain. To address this, Alpenglow
proposed adding finalization certificate to the block footer. This certificate
consists of BLS-aggregated signatures representing votes from validators
collectively controlling a significant amount of stake. They offer a concise,
verifiable record that blocks have been finalized correctly, enabling reliable
RPC status reporting and supporting a broad range of downstream uses.

## Dependencies

- Alpenglow is specified in [SIMD 326](https://github.com/solana-foundation/solana-improvement-documents/pull/326)

- Block footer is specified in [SIMD 307](https://github.com/solana-foundation/solana-improvement-documents/pull/307)

## New Terminology

- **Finalization Certificate for Observability**: is a proof that a specified
block is finalized by aggregating specific types of votes. Note that the data
format is slightly different from Certificate in SIMD 326, because we are
combining *slow-finalization* and *notarization* certificates into one data
structure when necessary. See details in *Finalization Certificate for
Observability data structure*.

## Detailed Design

### Data Layout in Block Footer

#### Finalization Certificate for Observability data structure

```rust
pub struct VotesAggregate {
    signature: BLSSignatureCompressed,
    bitmap: Vec<u8>,
}

pub struct FinalCertificate {
    pub slot: Slot,
    pub block_id: Hash,
    pub final_aggregate: VotesAggregate,
    pub notar_aggregate: Option<VotesAggregate>,
}
```

Where `Slot` is u64, `Hash` is [u8; 32], and `BLSSignatureCompressed` is
specified in `bls-signatures`, it is a 96 byte array.

Please refer to `solana-signer-store` for bitmap format. We expect to be using
`solana-signer-store 0.1.0` for the Alpenglow launch. Only base2-encoding
will be used in either bitmap. When `notar` is `None`, this is a fast
finalization cert. Otherwise it’s a slow finalization cert.

Block Footer Extension

```rust

pub struct BlockFooterV1 {
    pub bank_hash: Hash,                               // Introduced in V1 (SIMD-0298)
    pub block_producer_time_nanos: u64,                // Introduced in V1 (SIMD-0307)
    pub block_user_agent: Vec<u8>,                     // Introduced in V1 (SIMD-0307)
    pub final_cert: Option<FinalCertificate>,          // New, in this SIMD
}
```

**Note on Versioning and Field Ordering**: While adding fields to the footer
would typically warrant a version increment, we maintain `footer_version=1`
for simplicity.

We only make these atypical changes in light of the fact that, as of November
2025, clients do not yet disseminate block footers or block markers, making
this an appropriate time to modify the version 1 format before widespread
adoption.

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
| final_cert_present           (1 byte) | ← NEW
+---------------------------------------+
| final_cert                 (variable) | ← NEW
+---------------------------------------+
```

If the `final_cert_present` is 0, then there is no `final_cert` following it,
otherwise it is 1.

#### FinalCertificate Serialization

If `final_cert` is present, it is serialized as follows:

```
+---------------------------------------+
| slot                        (8 bytes) |
+---------------------------------------+
| block_id                   (32 bytes) |
+---------------------------------------+
| final_aggregate_signature  (96 bytes) |
+---------------------------------------+
| final_aggregate_bitmap_len  (2 bytes) |
+---------------------------------------+
| final_aggregate_bitmap     (variable) |
+---------------------------------------+
| notar_aggregate_present      (1 byte) |
+---------------------------------------+
| notar_aggregate_signature  (variable) |
+---------------------------------------+
| notar_aggregate_bitmap_len (variable) |
+---------------------------------------+
| notar_aggregate_bitmap     (variable) |
+---------------------------------------+
```

If the `notar_aggregate_present` is 0, then there are no
`notar_aggregate_signature`, `notar_aggregate_bitmap_len`, and
`notar_aggregate_bitmap` following it. Otherwise, we will have
`notar_aggregate_signature` as 96 bytes array,
`notar_aggregate_bitmap_len` as u16 in little endian, and
`notar_aggregate_bitmap` following that.

### Field Population by leader

While producing a block at slot `s`, the leader should include the finalization
certificate corresponding to the highest slot available `t < s`. In the usual
case with no skipped slots, this will be the certificate for `s − 1`, though
the leader ultimately decides which certificates to include.

If a fast finalization certificate is available, the leader should include only
fast finalization cert in the `final` field. Otherwise, the leader should
include the slow finalization cert in the `final` field and the notarization
cert in the `notar` field.

### Field Validation by non-leaders

Validators MUST enforce the following rules:

1. Type Constraints: When `notar` field is `None`, `final` field must be an
aggregate of notarization votes. Otherwise the `final` field must be an
aggregate of finalization votes, and the `notar` field must be an aggregate of
notarization votes.

2. BLS Validity: All certificates provided must pass BLS signature verification.

3. Consensus Thresholds: Each certificate must meet the consensus thresholds
specified by the Alpenglow protocol (SIMD-0326, https://www.anza.xyz/alpenglow-1-1).

Any violation should cause the slot to be marked dead, and the remainder of the
leader window should be skipped.

### RPC change to Validator Delinquent status

The RPC layer will read the parsed certificates from bank replay and use the
bitmaps embedded in those certificates to update each validator’s voting
status.

To interpret the bitmaps, RPC can pull the BLS public keys from vote
accounts and retrieve each account’s stake from the bank. Validators are then
ranked by sorting first by stake in descending order, and breaking ties by
public keys in ascending order. This deterministic ordering maps cleanly onto 
the bitmap positions, allowing the RPC code to identify exactly which staked
validators participated in a given vote.

## Alternatives Considered

**Transaction-Based Distribution**: Rejected because the execution overhead was
too high.

**Directly using Certificate format in Consensus Pool**: Rejected because under
the new format it's easier to enforce the rule that `FinalizationCertificate`
contains either fast or slow finalization certificates.

**Use base-3 encoding in Certificate**: We can use base-3 encoding in `Skip` or
`NotarizeFallback` certs in Alpenglow consensus pool because the pool checks to
make sure there will never be a `(true, true)` combination where two different
votes are presented for any validator. This is not true in this case. In most
of the cases, validators will send both `Notarize` and `Finalize` for a block.

## Impact

Invalid certificates will cause the block to be marked dead.

## Security Considerations

N/A

## Backwards Compatibility

Not backward compatible.
