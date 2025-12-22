---
simd: '0436'
title: Reduce Rent-Exempt Minimum Balance by 2x
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-12-22
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Reduce `lamports_per_byte_year` by half, which is equivalent
to reducing the rent-exempt minimum balance by half for all
new and existing accounts.

Requires SIMD-0392: Relaxation of post-execution min_balance check,
to allow for subsequent rent increases if necessary.

## Motivation

Minimum account balance depends on an arbitrary constant set
years ago that has since increased significantly in real terms
due to increases in the SOL price. This has made state allocation
cost more expensive on mainnet beta than on competitors, with no
compelling justification provided by real resource costs.

## New Terminology

N/A

## Detailed Design

On feature activation, set the effective `lamports_per_byte_year`
to `current_value / 2` in the runtime, Rent sysvar, and RPC
helpers (e.g., `getMinimumBalanceForRentExemption`).

Note: SIMD-0194 changed `lamports_per_byte_year` from `3480`
to `6960` and reduced `exemption_threshold` from `2.0` to
`1.0`, effectively deprecating the `exemption_threshold`
while keeping the actual rent-exempt minimum balance constant.
This proposal reduces `lamports_per_byte_year` back to `3480`.

```
ACCOUNT_STORAGE_OVERHEAD = 128
lamports_per_byte_year = 3480 // reduced from 6960 by this proposal
exemption_threshold = 1.0
effective_size = ACCOUNT_STORAGE_OVERHEAD + data_size_bytes
min_balance(data_size) = effective_size 
                            * lamports_per_byte_year
                            * exemption_threshold
                       = effective_size * 3480
```

## Alternatives Considered

This proposal is the first step towards further rent reduction and
doesn't prevent adoption of alternative solutions like a supervisory
controller (SIMD-0389). A 2x reduction is picking the low hanging
fruit by modestly reducing the extremely inflated rent cost on mainnet
beta today.

## Impact

- Lower rent for app developers. Existing accounts and programs using
  the higher rent value will be unaffected, besides being allowed to
  reduce the balance to the new minimum.
- Validators: a potential increase in state growth. In the unlikely
  case of excessive state growth, rent can be increased back to the
  legacy value (0392 allows this without significant disruption to
  existing accounts.)

## Security Considerations

N/A

## Backwards Compatibility

Rent reduction is strictly a relaxation of existing constraints so
all existing program logic will continue to work as before.
