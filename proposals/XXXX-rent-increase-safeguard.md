---
simd: 'XXXX'
title: Safeguard for rent-exempt minimum increase
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-12-22
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Protective feature to increase rent-exempt minimum balance back
to the legacy value of `6960` per-byte. This should only be
activated in response to excessive state growth observed on mainnet
beta.

## Motivation

Reducing rent without a built-in safeguard to respond to excessive
state growth is inherently risky. This proposal is designed to
provide a rent increase feature gate proactively so core developers
can respond to issues more promptly.

## New Terminology

N/A

## Detailed Design

Reset `lamports_per_byte_year` to the legacy value of `6960`.

```
ACCOUNT_STORAGE_OVERHEAD = 128
lamports_per_byte_year = 6960 // post-activation
exemption_threshold = 1.0
effective_size = ACCOUNT_STORAGE_OVERHEAD + data_size_bytes
min_balance = effective_size 
                            * lamports_per_byte_year
                            * exemption_threshold
                       = effective_size * 6960
```

## Alternatives Considered

SIMD-0389 introduces a supervisory controller that will detect
and respond to excessive state growth automatically. Because that
alternative is yet to be accepted, the manual intervention described
in this proposal can
allow for safely shipping moderate rent reduction sooner while
further reduction can be withheld until the controller is shipped.

## Impact

- New account creations and allocations will be subject to the
  increased rent value. SIMD-0392 maintains usability of existing
  accounts that are made sub-exempt after the rent increase.

## Security Considerations

N/A

## Backwards Compatibility

SIMD-0392 grandfathers in existing accounts so they will continue
to be valid after a rent increase.
