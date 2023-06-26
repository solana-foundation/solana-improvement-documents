---
simd: '00xx'
title: Add receipt tree to gossip
authors:
  - Anoushk Kharangate (Tinydancer)
  - Harsh Patel (Tinydancer)
category: Standard/Meta
type: Networking
status: Draft
created: (2023-06-27)
---

## Summary
Add Transaction receipts root and attestation to the CRDS enum

## Motivation
Since there is interest in supporting light clients that are able to verify transactions locally without relying on RPC requests, 
out of the several methods already discussed earlier, modifying gossip would be the least invasive to the core protocol.

## Alternatives Considered

## New Terminology

Is there any new terminology introduced with this proposal?

## Detailed Design
We propose adding a new field to 
```
pub enum CrdsData {
    LegacyContactInfo(LegacyContactInfo),
    Vote(VoteIndex, Vote),
    LowestSlot(/*DEPRECATED:*/ u8, LowestSlot),
    LegacySnapshotHashes(LegacySnapshotHashes),
    AccountsHashes(AccountsHashes),
    EpochSlots(EpochSlotsIndex, EpochSlots),
    LegacyVersion(LegacyVersion),
    Version(Version),
    NodeInstance(NodeInstance),
    DuplicateShred(DuplicateShredIndex, DuplicateShred),
    SnapshotHashes(SnapshotHashes),
    ContactInfo(ContactInfo),
  + TransactionReceipt(TransactionReceipt)  
}
```
where
```
pub struct TransactionReceipt{
  attestation: Signature,
  root: Hash,
}
```
i

## Impact
CRDS will have transaction receipts which can be subscribed to by light clients and this will be consistent across the entire cluster.
Verifying receipts by comparing the locally computed receipt with the cluster wide receipt would be much more convenient.

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

Does the feature introduce any breaking changes? All incompatibilities and
consequences should be listed.
