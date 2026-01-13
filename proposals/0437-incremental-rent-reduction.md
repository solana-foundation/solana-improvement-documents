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

This proposal defines five incremental reductions to the
`lamports_per_byte` constant. Each reduction is controlled by its own
feature gate. The values are absolute, starting from the current value
of 6960.

### Reduction Schedule

The incremental percentage represents the portion of the total target
reduction (6264 lamports, from 6960 to 696) applied at each step. The
target `lamports_per_byte` values are derived by applying the cumulative
percentage reduction to the total reduction amount and truncating the
result to the nearest integer.

| Step | Target        | Incr % | Cumul % | Eff Reduc |
|------|---------------|--------|---------|-----------|
| 1    | 6333          | 10%    | 10%     | 9%        |
| 2    | 5080          | 20%    | 30%     | 27%       |
| 3    | 2575          | 40%    | 70%     | 63%       |
| 4    | 1322          | 20%    | 90%     | 81%       |
| 5    | 696           | 10%    | 100%    | 90%       |

### Operational Details

Each reduction step is controlled by its own feature gate and must meet
specific activation criteria.

| Step | Feature Gate | Criteria |
|------|--------------|----------|
| 1    | TBD          | Applied immediately |
| 2    | TBD          | See [Criteria 2](#step-2-criteria) |
| 3    | TBD          | See [Criteria 3](#step-3-criteria) |
| 4    | TBD          | See [Criteria 4/5](#step-45-criteria) |
| 5    | TBD          | See [Criteria 4/5](#step-45-criteria) |

#### Step 2 Criteria

Depends on (1).

Average net increase per-day following activation of (1):

- In account data size: does not exceed 250MB (measured over at least 3
  weeks).
- In number of accounts: does not exceed 1.5M (measured over at least 3
  weeks).

If after the first 3 weeks these conditions aren't met then the timeline
is extended until the averages since the activation of (1) satisfy the
requirements.

#### Step 3 Criteria

Activation of SIMD-0389 (supervisory controller).

Does not depend on activation of (1) or (2), meaning that if 0389 is
activated then the schedule can skip directly to (3).

#### Step 4/5 Criteria

The supervisory controller (SIMD-0389) has not been engaged for at
least 3 continuous weeks.

(4) depends on activation of (3) and (5) depends on activation of (4).

### Implementation

On activation of each feature, the effective `lamports_per_byte` is
updated in the bank and rent sysvar
(`SysvarRent111111111111111111111111111111111`), followed by updating
`DEFAULT_LAMPORTS_PER_BYTE` in all relevant SDKs post-activation.

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

N/A

## Impact

- Lower rent for app developers. Existing accounts and programs using
  the higher rent value will be unaffected, besides being allowed to
  reduce the balance to the new minimum.
- Validators: a potential increase in state growth. In the case of
  excessive state growth, rent can be increased back to the legacy
  value (0392 allows this without significant disruption to existing
  accounts). SIMD-0389 does this automatically.

## Security Considerations

N/A

## Backwards Compatibility

Rent reduction is strictly a relaxation of existing constraints so
all existing program logic will continue to work as before.
