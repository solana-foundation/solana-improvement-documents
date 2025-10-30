---
simd: '0390'
title: Arbitrary-Byte Seeds for Seeded Addresses
authors:
  - Kevin Heavey
category: Standard
type: Core
status: Idea
created: 2025-10-30
---

## Summary

Permit all seeded address derivation helpers and instructions to accept raw
byte slices instead of UTF-8 strings, eliminating redundant validation and
unlocking the ability to pass binary seeds directly.

## Motivation

Seeded derivation is widely used by on-chain programs and client tooling. The
current SDK and runtime APIs expose these helpers as `&str`/`String` even though
the underlying implementation immediately converts the seed into bytes. That
choice produces two concrete issues:

- **Wasted compute:** the UTF-8 validation that runs before every call consumes
  cycles for no benefit.
- **Artificial constraints:** callers cannot pass arbitrary binary data (for
  example an `Address`) without first encoding it as UTF-8, adding conversions and
  allocations that frequently also increase compute usage onchain and offchain.

Removing the string requirement avoids paying for validation we do not use and
reduces friction for both programs and clients that already have a byte slice
handy. The relaxed behavior is activated via a feature gate so that all nodes
transition in lockstep.

## Dependencies

None.

## New Terminology

None.

## Detailed Design

### SDK and Program APIs

SDK changes are here: https://github.com/anza-xyz/solana-sdk/pull/361

tl;dr Expect bytes instead of strings, but accommodate passing strings where possible.

### Runtime Changes

1. In Agave's `system_processor.rs` and `vote_processor.rs`, remove conversions
   that re-create strings from instruction data. Instead, operate directly on
   `Vec<u8>` seed payloads.
2. Behind a feature gate, stop deserializing seeds as `String`; this removes the
   implicit UTF-8 validation performed by the serializer when reconstructing
   strings. While the feature is inactive, keep the string-based path to preserve
   consensus with older nodes.

### Firedancer

Firedancer mirrors the SDK requirement today by validating UTF-8 when it parses
seeded instructions. The implementation must remove those checks and treat the
seed as opaque bytes while enforcing the same 32-byte maximum.

### Feature Activation and Rollout

Introduce a new runtime feature (e.g., `allow_arbitrary_seed_bytes`) that guards
the relaxed validation. Until the feature activates, Agave and Firedancer
continue to reject non-UTF-8 seeds.

## Alternatives Considered

- **Add parallel byte-oriented APIs:** Introducing `*_with_seed_bytes`
  variants would retain the old string interfaces, but would increase
  API surface area and preserve a "wrong way to do it".
  It would also keep the unwanted validation on the original functions,
  meaning transactions would have to opt in to gain the benefit.

## Impact

- **Program developers:** Client-side code that already handled UTF-8 continues to
  work unchanged, while binary seed workflows (e.g. using an `Address` as a seed)
  become straightforward.
- **Validators:** Runtime processing becomes faster because it no
  longer validates UTF-8 for every seeded instruction.
- **Core contributors:** Simplifies system and vote program code paths by
  removing redundant conversions.

## Security Considerations

The proposal does not relax any cryptographic or length constraints on seeds.
Derivation logic and collision resistance are unchanged, and the seed remains
limited to 32 bytes. Removing UTF-8 validation does not introduce new attack
vectors because the runtime never required the seed to be human-readable.

## Drawbacks *(Optional)*

Breaking API changes, while somewhat mitigated with clever
type signatures, are still real.

## Backwards Compatibility *(Optional)*

The serialized instruction format is preserved, so existing transactions remain
valid. While the feature gate is inactive, the runtime keeps enforcing UTF-8 to
match legacy nodes. Once the feature activates, validators must run versions
that understand the new rules, otherwise they will reject transactions with
non-UTF-8 seeds.
