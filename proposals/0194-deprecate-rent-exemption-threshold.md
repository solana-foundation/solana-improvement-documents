---
simd: '0194'
title: Deprecate Rent Exemption Threshold
authors:
  - Dean Little (@deanmlittle)
  - Leonardo Donatacci (@L0STE)
  - febo (Anza)
category: Standard
type: Core
status: Accepted
created: 2024-11-13
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal aims to eliminate the use of floating-point operations to 
determine whether an account is rent exempt or not in programs by deprecating
the use of the `exempt_threshold` (`f64`) value. 

More specifically, rename `lamports_per_byte_year` to `lamports_per_byte`, 
change default value from `3480` to `6960`, change `exemption_threshold` to 
`1.0` and deprecate it from the protocol. This will enable us to remove `f64` 
math from all Rent-related SDKs and onchain programs moving forward.

## Motivation

Emulating floating point math is expensive inside of SVM due to the lack of 
native floating point number support. This makes calculating the rent exempt 
threshold cost `~248` more CUs than it would if we were to simply use a `u64`. 
This is due to the `exemption_threshold` (`f64`), which is currently set to 
`2.0` years. Since rent exemption is no longer time-based, we have the 
opportunity to make this commonly used calculation much more performant and 
simplify our tooling without affecting the existing API. It also simplifies any 
further changes we may want to make to Rent down the line.

## New Terminology

`lamports_per_byte` - the number of lamports required to pay for 1 byte of 
account storage.

## Detailed Design

Set the value of `DEFAULT_EXEMPTION_THRESHOLD` from `2.0` to `1.0`, and 
deprecate it from the protocol.

```rs
pub const DEFAULT_EXEMPTION_THRESHOLD: f64 = 1.0;
```

Set `DEFAULT_LAMPORTS_PER_BYTE` from its current value of `3480` (the `u64` 
value of `1_000_000_000 / 100 * 365 / (1024 * 1024)`), to `6960`; double its 
current `u64` value, to counteract reducing the exemption threshold above.

```rs
pub const DEFAULT_LAMPORTS_PER_BYTE: u64 = 6960;
```

Rename `lamports_per_byte_year` in the Rent account to `lamports_per_byte` to 
reflect that Rent is no longer time-based and officially deprecate 
`exemption_threshold`.

```rs
pub struct Rent {
    pub lamports_per_byte: u64,
    #[deprecated(since = "2.X.X", note = "Use only `lamports_per_byte` 
instead")]
    pub exemption_threshold: f64,
    pub burn_percent: u8,
}
```

 Remove all `f64` math from Rent calculations in all SDKs and onchain programs 
moving forwards, replacing it with simple `u64` math, ie:

```rs
/// Minimum balance due for rent-exemption of a given account data size.
pub fn minimum_balance(&self, data_len: usize) -> u64 {
    (ACCOUNT_STORAGE_OVERHEAD + data_len as u64) * self.lamports_per_byte
}
```

Validator implementations should stop using `exemption_threshold` and only use
the `lamports_per_byte` value. Any existing program using the current 
implementation will be unaffected.

## Alternatives Considered

- Leave things as they are.
  - Calculating rent exemption on a program remains an 
expensive operation.

- Allow users to make the assumption that `2.0` will remain stable and do `u64` 
math themselves.
  - Risk of the protocol changing on them.

- Perform the "conversion" ouside the VM and change the type of 
`exemption_threshold` to a `u64` using a new `RentV2` struct.
  - Requires a new syscall and sysvar.

## Impact

This change will significantly improve the compute unit (CU) consumption of
new onchain programs using updated SDKs programs that calculate the minimum 
lamports balance required for rent exemption. Currently, the calculation of the 
minimum balance for rent exemption (`Rent::minimum_balance`) consumes
approximately `256` CUs; with the proposed change, this will be reduced to
just `8` CUs. Existing programs will not be impacted.

## Security Considerations

None.

## Drawbacks

None.

## Backwards Compatibility

This feature is backwards compatible. It does not change the absolute value 
required for an account to be rent exempt and the current way of calculating 
the threshold will continue to evaluate to the same value. Any deployed program 
will be unaffected.
