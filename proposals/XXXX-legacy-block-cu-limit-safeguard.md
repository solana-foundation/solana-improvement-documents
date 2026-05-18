---
simd: 'XXXX'
title: Legacy Block CU Limit Safeguard
authors:
  - Igor Durovic (Anza)
category: Standard
type: Core
status: Idea
created: 2026-05-18
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Introduce a latent feature gate that restores `Max Block Units` to the legacy
value of `60_000_000` CUs. This safeguard is intended to be shipped before or
alongside any SIMD
that raises the block CU limit high enough that it no longer serves as the
primary replay bound.

## Motivation

Raising `Max Block Units` substantially under Alpenglow increases risk around
skip rate, replay lag, denial-of-service, and geographic centralization. A
pre-shipped safeguard allows the network to restore the legacy `60_000_000` CU
block limit quickly if those risks materialize.

This proposal is intended to safeguard the companion proposal in
[SIMD-0538](https://github.com/solana-foundation/solana-improvement-documents/pull/538).

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0326]: [Alpenglow](https://github.com/solana-foundation/solana-improvement-documents/pull/326)**

    Alpenglow is the environment in which replay time is expected to be bounded
    primarily by timeout behavior rather than by the block CU limit.

## New Terminology

N/A

## Detailed Design

This proposal introduces a feature gate that sets `Max Block Units` to the
legacy block CU limit of `60_000_000` when active.

This proposal does not change:

- transaction-level CU metering
- transaction-level CU limit enforcement
- any block-level limits other than `Max Block Units`

The safeguard gate is intended to coexist with a separate feature gate that
raises `Max Block Units`.

If both feature gates exist, clients MUST apply the following precedence:

| Large-block gate | Safeguard gate | Effective `Max Block Units` |
|------------------|----------------|-----------------------------|
| inactive | inactive | `60_000_000` |
| active | inactive | raised value |
| inactive | active | `60_000_000` |
| active | active | `60_000_000` |

This means the safeguard gate always overrides the large-block gate.

The safeguard gate MAY be activated even if the large-block gate is inactive.
In that case it is effectively a no-op and keeps `Max Block Units` at the
legacy value of `60_000_000`.

Validator clients SHOULD ship this gate before or alongside activation of any
proposal that materially raises `Max Block Units`.

## Alternatives Considered

- Ship the large-block proposal without a latent rollback gate

  This leaves the network without a protocol-level rollback path other than a
  new emergency SIMD and coordinated client release.

- Use only client-local mitigations

  Local mitigations are still useful, but they do not provide a uniform
  protocol-level way to restore the legacy replay bound across the network.

## Impact

- No effect until the safeguard gate is activated.
- Provides a protocol-level rollback path for large-block operation.
- Reduces the operational cost of responding to elevated skip rate, replay
  instability, or centralization pressure after a block limit increase.

## Security Considerations

- Benefits:
  - Provides a rapid protocol-level response to elevated skip rate.
  - Provides a rollback path if larger blocks increase replay lag,
    denial-of-service pressure, or geographic centralization pressure.

- Risks:
  - This safeguard still requires coordinated activation.
  - If activated too late, the network may already have experienced sustained
    degradation.
  - If activated unnecessarily, the network gives up throughput and block
    capacity.

- Implementation requirements:
  - The safeguard must have higher precedence than any large-block feature
    gate.
  - Clients should test all activation combinations explicitly, especially the
    case where both gates are active.

## Backwards Compatibility

Before activation, this proposal has no effect.

After activation, validators that do not implement the safeguard gate may
accept blocks that upgraded validators reject. This proposal therefore requires
coordinated rollout and activation like any other consensus-changing feature
gate.
