---
simd: '0XXX'
title: Deprecate Rent Exemption Threshold
authors:
  - Dean Little (@deanmlittle)
  - Leonardo Donatacci (@L0STE)
  - Febo (@0x_febo)
category: Standard/Meta
type: Core
status: Draft
created: 2024-11-13
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Rename `lamports_per_byte_year` to `lamports_per_byte`, change default value from 3480 to 6960, change `exemption_threshold` to `1.0f64` and deprecate it from the protocol, enabling us to remove `f64` math from rent calculation and all rent-related SDKs in onchain programs moving forward.

## Motivation

Emulating floating point math is expensive inside of SVM due to the lack of native floating point number support. This makes calculating the rent exempt threshold cost ~248 more CUs than it would if we were to simply use a u64. This is due to the `exemption_threshold`, which is currently set to `2.0` years. Since rent exemption is no longer time-based, we have the opportunity to make this commonly used calculation much more performant and simplify our tooling without affecting the existing API. It also simplifies any further changes we may want to make to rent down the line.

## New Terminology

`lamports_per_byte` - the number of lamports required to pay for 1 byte of account storage.

## Detailed Design

Half the value of `DEFAULT_EXEMPTION_THRESHOLD` from `2.0` to `1.0`, and deprecate it from the protocol.

```rs
pub const DEFAULT_EXEMPTION_THRESHOLD: f64 = 1.0;
```

Set `DEFAULT_LAMPORTS_PER_BYTE` from its current value of `3480` (the u64 value of `1_000_000_000 / 100 * 365 / (1024 * 1024)`), to `6960`; double its current u64 value, to counteract halving the exemption threshold above.

```rs
pub const DEFAULT_LAMPORTS_PER_BYTE: u64 = 6960;
```

Rename `lamports_per_byte_year` in `Rent` to `lamports_per_byte` to reflect that rent is no longer time-based.

```rs
pub struct Rent {
    pub lamports_per_byte: u64,
    pub exemption_threshold: f64,
    pub burn_percent: u8,
}
```

Officially deprecate `exemption_threshold` and remove all f64 math from rent calculations in all SDKs and onchain programs moving forwards, replacing it with simple u64 math, ie:

```rs
/// Minimum balance due for rent-exemption of a given account data size.
pub fn minimum_balance(&self, data_len: usize) -> u64 {
    (ACCOUNT_STORAGE_OVERHEAD + data_len as u64) * self.lamports_per_byte
}
```

## Alternatives Considered

- Leave things as they are.

- Allow users to make the assumption that `2.0` will remain stable and do u64 math themselves at risk of the protocol changing.

- Don't rename `lamports_per_byte_year`

- Don't change `exemption_threshold` and instead ossify it at `2.0f64`

- Bundle changes to rent values with existing rent change proposals to avoid multiple SIMDs

## Impact

New onchain programs using updated SDKs will use far fewer CUs when calculating rent exemption. Calculating rent exemption itself will become simpler and more reliable. Existing programs will not be impacted.

## Security Considerations

None.

## Drawbacks

None.

## Backwards Compatibility

This feature is backwards compatible.
