---
simd: '0646'
title: Disable Legacy and v0 Transaction Formats
authors:
  - Andrew Fitzgerald (Anza)
category: Standard
type: Core
status: Draft
created: 2026-09-17
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Disable the legacy and v0 transaction formats. After feature activation,
these transactions are dropped during block production and rejected during
replay as if they failed deserialization.

## Motivation

With v1 transactions active, supporting legacy and v0 formats adds complexity
to transaction ingestion and processing. Keeping these formats also requires
continued support for address lookup tables and compute budget instructions.
Requiring v1 transactions removes the need for these mechanisms and for
supporting multiple transaction formats.

## Dependencies

- [SIMD-0385: Transaction V1 Format](0385-transaction-v1.md)
- [SIMD-0326: Alpenglow](0326-alpenglow.md). Alpenglow must be active before
  disabling legacy transactions, since vote transactions use the legacy format.

## New Terminology

None.

## Detailed Design

A new feature gate disables legacy and v0 transactions. After activation,
validators MUST reject legacy and v0 transactions during replay as if they
failed deserialization. A block containing either format is invalid.

After activation, leaders MUST drop legacy and v0 transactions during block
production. Validation of v1 transactions is unchanged.

### Activation Timeline

The v1 transaction format activated on mainnet on September 15, 2026.
Mainnet activation of this feature MUST be at least 180 days after that date,
giving developers approximately six months to migrate. The feature MUST NOT
activate on mainnet before March 14, 2027.

Activation MUST also wait until Alpenglow is active on the cluster.

## Alternatives Considered

Continue supporting legacy and v0 transactions indefinitely. This retains the
complexity of multiple transaction formats, address lookup tables, and compute
budget instructions.

## Impact

Transaction producers, including wallets, SDKs, and validators, must migrate
to v1 before activation. Applications using v0 address lookup tables must
include the account addresses directly in their v1 transactions.

## Security Considerations

None at this time.

## Backwards Compatibility

This is a breaking change for producers of legacy and v0 transactions.

## Conformance

Verify that otherwise valid legacy and v0 transactions are accepted before
activation, are dropped during block production after activation, and invalidate
blocks containing them during replay after activation. Verify that v1
transactions remain accepted across activation.
