---
simd: '0437'
title: Incremental Reduction of lamports_per_byte to 696
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-12-22
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Reduce `lamports_per_byte` incrementally from 6960 to 3480, then to 1740, and
finally to 696. The first reduction to 3480 lines up with SIMD-0436; this proposal
defines the subsequent steps and the overall schedule to reach the final 696 target.

Requires SIMD-0392: Relaxation of post-execution min_balance check,
to allow for subsequent rent increases if necessary.

## Motivation

Minimum account balance depends on an arbitrary constant set
years ago that has since increased significantly in real terms
due to increases in the SOL price. This has made state allocation
cost more expensive on mainnet beta than on competitors, with no
compelling justification provided by real resource costs.

This reduction is split into multiple steps to allow for gradual
observation of state growth and to allow for controversial steps
to be debated separately. SIMD-0436 handles the first, least controversial
reduction to 3480. Subsequent reductions are proposed here to achieve
the target of 696.

## New Terminology

N/A

## Detailed Design

This proposal defines three incremental reductions to the `lamports_per_byte` constant.
Each reduction is controlled by its own feature gate. The values are absolute,
starting from the current value of 6960.

| Step | Target `lamports_per_byte` | Feature Gate |
|------|----------------------------|--------------|
| 1    | 3480                       | (SIMD-0436)  |
| 2    | 1740                       | TBD          |
| 3    | 696                        | TBD          |

On activation of each feature, the effective `lamports_per_byte` is updated
in the bank and rent sysvar
(`SysvarRent111111111111111111111111111111111`), followed by
updating  `DEFAULT_LAMPORTS_PER_BYTE` in all relevant SDKs
post-activation.

The first step (reduction to 3480) is identical to SIMD-0436. If SIMD-0436 is
already activated, Step 1 of this proposal is already satisfied.

```
ACCOUNT_STORAGE_OVERHEAD = 128
effective_size = ACCOUNT_STORAGE_OVERHEAD + acc.data_size_bytes
min_balance = effective_size * lamports_per_byte

// Original:   min_balance = 6960 * effective_size
// After Step 1 (0436): min_balance = 3480 * effective_size
// After Step 2:        min_balance = 1740 * effective_size
// After Step 3:        min_balance = 696  * effective_size
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
