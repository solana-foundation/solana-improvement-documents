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
There's no security benefit, and in fact security is reduced when the number of
shreds is low.

## New Terminology

N/A

## Detailed Design

A sender should always produce 32 data shreds + 32 coding shreds per FEC set
(this is currently already happening).

Receivers currently accept FEC sets with variable number of shreds.

If `enforce_32_data_shreds: <PUBKEY>`
is active, then any FEC set with a number of shreds different than 32 data + 32 coding
will be dropped on ingest.

## Alternatives Considered

Leave as is.

## Impact

Clients will no longer accept FEC sets (thus blocks) with any number of shreds
different than 32 data + 32 coding.

## Security Considerations

Security is improved since the (minimum) number of shreds is now 32 + 32.

## Backwards Compatibility

This feature is not backwards compatible.
