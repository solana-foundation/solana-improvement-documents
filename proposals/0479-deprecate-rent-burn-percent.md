---
simd: '0479'
title: Deprecate Rent Burn Percent
authors:
  - Dean Little (@deanmlittle)
category: Standard
type: Core
status: Accepted
created: 2026-03-03
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Set `burn_percent` in `Rent` to `0` and deprecate it from the protocol.

## Motivation

The `burn_percent` value in `Rent` is no longer used anywhere in the protocol 
and has already been flagged as deprecated in `solana-sdk@4.0.1`. 
Unfortunately, its value being set in the Rent sysvar account recently 
resulted in a network divergence on testnet. With many upcoming changes to 
Rent, it would be ideal to clean this up by zeroing out the sysvar value 
moving forwards.

## New Terminology

N/A

## Detailed Design

Set the value of `DEFAULT_BURN_PERCENT` to `0`, update `burn_percent` value in 
`Rent` sysvar and `RentCollector` to `0`.

```rs
pub const DEFAULT_BURN_PERCENT: u8 = 0;
```

## Alternatives Considered

- Leave current value at `50`

## Impact

Reduce footguns in future Rent changes by removing a vestigial feature of the 
protocol.

## Security Considerations

None.

## Drawbacks

None.

## Backwards Compatibility

This feature is backwards compatible.
