---
simd: '0437'
title: Reduce Rent-Exempt Minimum Balance by 5x
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-12-22
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Reduce `lamports_per_byte_year` by 5x (80%), which is equivalent
to reducing the rent-exempt minimum balance by 5x for all
new and existing accounts. Combined with SIMD-0436, this amounts
to a 10x total reduction.

Requires SIMD-0392: Relaxation of post-execution min_balance check,
to allow for subsequent rent increases if necessary.

## Motivation

Minimum account balance depends on an arbitrary constant set
years ago that has since increased significantly in real terms
due to increases in the SOL price. This has made state allocation
cost more expensive on mainnet beta than on competitors, with no
compelling justification provided by real resource costs.

This is split out from SIMD-0436 because it's less clear how safe
another 5x reduction on top of a 2x reduction would be. SIMD-0389
is designed to address this risk by introducing a supervisory
controller to detect and respond to excessive state growth
automatically.

## New Terminology

N/A

## Detailed Design

On feature activation, set the effective `lamports_per_byte_year`
to `current_value / 5` in the runtime, Rent sysvar, and RPC
helpers (e.g., `getMinimumBalanceForRentExemption`).

Note: SIMD-0194 changed `lamports_per_byte_year` from `3480`
to `6960` and reduced `exemption_threshold` from `2.0` to
`1.0`, effectively deprecating the `exemption_threshold`
while keeping the actual rent-exempt minimum balance constant.

```
ACCOUNT_STORAGE_OVERHEAD = 128
exemption_threshold = 1.0
effective_size = ACCOUNT_STORAGE_OVERHEAD + acc.data_size_bytes
min_balance = effective_size 
                            * lamports_per_byte_year
                            * exemption_threshold

// prior to rent-reduction: min_balance = 6960 * effective_size
// with this proposal alone: min_balance = 1392 * effective_size
// combined with 0436: min_balance = 696 * effective_size
```

## Alternatives Considered

This proposal is intended to be combined with safety measures like
a supervisory controller (SIMD-0389).

## Impact

- Lower rent for app developers. Existing accounts and programs using
  the higher rent value will be unaffected, besides being allowed to
  reduce the balance to the new minimum.
- Validators: a potential increase in state growth. In the
  case of excessive state growth, rent can be increased back to the
  legacy value (0392 allows this without significant disruption to
  existing accounts). SIMD-0389 does this automatically.

## Security Considerations

N/A

## Backwards Compatibility

Rent reduction is strictly a relaxation of existing constraints so
all existing program logic will continue to work as before.
