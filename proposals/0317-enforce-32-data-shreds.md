---
simd: '0317'
title: Enforce 32 data + 32 coding shreds
authors:
  - Emanuele Cesena
category: Standard
type: Core
status: Review
created: 2025-07-08
feature:
---

## Summary

All clients currently send 32 data + 32 coding shreds, however Turbine still allows
to send and receive a variable number of shreds. With this change we will enforce
sending and receiving only 32 data + 32 coding shreds.

## Motivation

It is inconvinient to support many combinations of data + coding shreds.
Even the logic to validate if a shred index is valid or not is complex
because it requires to receive a coding shred for the FEC set to know the
index boundaries. With fixed 32 data + 32 coding shreds this logic becomes
trivial.

There's no security benefit in variable number of data + coding shreds,
and in fact security is reduced when the number of shreds is low.

With fixed 32 data + 32 coding shreds, equivocation detection is simplified
because it's sufficient to receive any two shreds in the same FEC set with
different Merkle roots and valid signatures.

## New Terminology

N/A

## Detailed Design

A sender should always produce 32 data shreds + 32 coding shreds per FEC set
(this is currently already happening).

Receivers currently accept FEC sets with variable number of shreds.

If `enforce_32_data_shreds: <PUBKEY>`
is active, then any FEC set with a number of shreds different than 32 data + 32 coding
will be dropped on ingest.

As a result, the FEC set payload must be exactly equal to 31840 bytes (with 995 bytes
of payload per data shred).

## Alternatives Considered

Leave as is.

## Impact

Clients will no longer accept FEC sets (thus blocks) with any number of shreds
different than 32 data + 32 coding.

## Security Considerations

Security is improved since the (minimum) number of shreds is now 32 + 32,
validating shred indexes is trivial and equivocation detection is simplified.

## Backwards Compatibility

This feature is not backwards compatible.
