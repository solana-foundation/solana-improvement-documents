---
simd: '0437'
title: Incrementally Reduce lamports_per_byte to 696
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-12-22
supersedes: 0436
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Reduce `lamports_per_byte` incrementally from 6960 to 696 via five steps:
6333, 5080, 2575, 1322, and 696. This proposal supersedes SIMD-0436,
providing a more granular reduction schedule.

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
to be debated separately. This proposal replaces the 2x reduction 
previously suggested in SIMD-0436 with a five-step schedule 
culminating in the final target of 696.

## New Terminology

N/A

## Detailed Design

This proposal defines five incremental reductions to the `lamports_per_byte` constant.
Each reduction is controlled by its own feature gate. The values are absolute,
starting from the current value of 6960.

| Step | Target `lamports_per_byte` | Feature Gate |
|------|----------------------------|--------------|
| 1    | 6333                       | TBD          |
| 2    | 5080                       | TBD          |
| 3    | 2575                       | TBD          |
| 4    | 1322                       | TBD          |
| 5    | 696                        | TBD          |

On activation of each feature, the effective `lamports_per_byte` is updated
in the bank and rent sysvar
(`SysvarRent111111111111111111111111111111111`), followed by
updating  `DEFAULT_LAMPORTS_PER_BYTE` in all relevant SDKs
post-activation.

```
ACCOUNT_STORAGE_OVERHEAD = 128
effective_size = ACCOUNT_STORAGE_OVERHEAD + acc.data_size_bytes
min_balance = effective_size * lamports_per_byte

// Original:      min_balance = 6960 * effective_size
// After Step 1:  min_balance = 6333 * effective_size
// After Step 2:  min_balance = 5080 * effective_size
// After Step 3:  min_balance = 2575 * effective_size
// After Step 4:  min_balance = 1322 * effective_size
// After Step 5:  min_balance = 696  * effective_size
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
