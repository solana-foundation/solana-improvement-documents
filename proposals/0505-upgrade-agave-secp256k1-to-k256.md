---
simd: "0505"
title: Upgrade Agave secp256k1 syscall precompile to k256 (Agave-specific)
authors:
  - Sam Kim
  - Zhenfei Zhang
category: Standard
type: Core
status: Review
created: 2026-03-27
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Transition the `secp256k1` precompile and syscall implementation in the Agave
validator client from the `libsecp256k1` crate to the `k256` crate. This is an
Agave-specific upgrade, but make this upgrade behind a feature gate for safety.

## Motivation

Currently, Agave utilizes the `libsecp256k1` crate for the `secp256k1`
precompile and syscall implementation. Because this crate is no longer actively
maintained, a transition to the `k256` crate has been under consideration for
some time.

However, swapping out cryptographic dependencies carries inherent consensus
risks; any differing behavior between the crates could result in a network fork.
While we believe we have a strong understanding of the behavioral differences
between the two crates, executing this transition behind a feature gate is the
safest approach to minimize risk.

## New Terminology

N/A

## Detailed Design

The implementation will introduce a new feature gate and maintain dual
implementations of the `secp256k1` operations to facilitate a safe transition.

- Feature Gate: Introduce a new feature gate named `secp256k1_use_k256`.
  - Client-Specific Behavior: Because this is an Agave implementation
    detail, this will be an empty (no-op) feature gate for other validator
    clients, such as Firedancer, requiring no functional changes on their end.
- Precompile Implementation: Retain the existing precompile function utilizing
  the `libsecp256k1` crate.
  - Add a new version of the precompile function utilizing the `k256` crate.
  - Introduce branching logic:
    - If `secp256k1_use_k256` is **inactive**, invoke the `libsecp256k1` version.
    - If `secp256k1_use_k256` is **active**, invoke the `k256` version.
- **Syscall Implementation:**
  - Apply the exact same dual-implementation and branching logic to the
    `secp256k1` syscall execution path.

## Alternatives Considered

The primary alternative is to replace the `libsecp256k1` crate with `k256`
directly, without guarding the transition behind a feature gate. While this
approach would slightly reduce implementation boilerplate, it carries
consensus risk. If there is any subtle, uncaught differing behavior
between the two cryptographic implementations, a direct upgrade would
result in a network fork.

## Impact

There is no intended functional or performance impact on the network. Assuming
our understanding of the behavioral differences between `libsecp256k1` and
`k256` is accurate, and the normalization logic is implemented correctly, the
execution results will be perfectly identical. The feature gate serves strictly
as an additional safety measure to prevent any unintended consensus divergence.

## Security Considerations

The primary security risk is a consensus failure leading to a network fork if
the `k256` implementation diverges from `libsecp256k1` in any edge cases. To
mitigate this, the new implementation will undergo extensive differential
fuzzing (via `solfuzz-agave`) and testing against known vectors to guarantee
exact execution parity before the feature gate is activated on mainnet.

## Backwards Compatibility

Prior to the activation of the feature gate, there are no changes to the
consensus rules or precompile behavior; Agave will continue to use the
legacy `libsecp256k1` implementation.

Post-activation, the new `k256` implementation is designed to perfectly
emulate the behavior of the current `libsecp256k1` version. It will remain
fully backwards compatible, and smart contracts or applications relying on
the `secp256k1` precompile or syscall will not require any modifications.
