---
simd: "0201"
title: Create Cluster Sysvar
authors:
  - keith@metaplex.foundation
category: Standard
type: Core
status: Review
created: 2024-11-22
feature:
---

## Summary

There should be a natively accessible Sysvar (like Rent or Clock) that provides
Cluster details in order to determine the specific SVM environment.

## Motivation

It is currently not possible to determine which cluster a Solana program is
executing on from within the program itself. This makes dynamic behavior based
on different SVM environments difficult without requiring multiple builds of a
single protocol. Native determination of the SVM cluster would improve DevEx and
diminish opportunities to introduce bugs by allowing more dynamic program execution.

## Alternatives Considered

- **SDK Abstraction** - It would be possible but difficult to maintain separate
SDKs per cluster or dynamic resolution based on genesis hash

## New Terminology

**ClusterDetails** - The sysvar structure which provides the relevant details
for a program to determine which cluster it's executing on.

## Detailed Design

The following Cluster type and ClusterDetails struct would provide sufficient
details for a program to properly determine the environment on which it's
executing. String tags are included because using genesis hash alone is
insufficient for clusters that don't guarantee permanence such as devnet,
testnet, or roll-ups.

```rust
/// The cluster type, usually the first three with an optional named
/// field for more diverse clusters (alt-devnets, roll-ups).
#[repr(C)]
pub enum ClusterType: {
    mainnet,
    devnet,
    testnet,
    other(String),
}

#[repr(C)]
pub struct ClusterDetails {
    /// The genesis hash for the cluster, guaranteed for mainnet but
    /// possibly variable for other cluster types.
    pub genesis_hash: Hash,
    /// A named tag for the SVM chain on which the program is executing.
    pub blockchain: String,
    /// Cluster Type
    pub cluster_type: ClusterType,
}
```

## Impact

Dynamic determination of cluster would allow programs to perform better
validation of cluster-specific feature such as variable protocol fees,
irreplicable PDAs or derivations, or variable features across different blockchains.

## Security Considerations

Certain named fields could be invalidated due to changes to the underlying SVM runtime. Program developers should be sure to validate and fail gracefully if unexpected changes occur.
