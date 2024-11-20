---
simd: 'XXXX'
title: Define CUs for Builtin instructions
authors:
  - Tao Zhu (Anza)
category: Standard
type: Core
status: Draft
created: 2024-11-20
feature:
supersedes: 
superseded-by:
extends:
---

## Summary

Builtin programs should consume a predefined number of CUs for each instruction.

## Motivation

To correct account how many CUs builtin instructions should consume from VM's
CU meter, each builtin instructions should be individually meansured for their
execution cost.

## New Terminology

None

## Detailed Design

1. Statically define each builtin instruction's execution cost, When the virtual
machine (VM) invokes a builtin instruction, the defined DEFAULT_COMPUTE_UNITS
is consistently deducted from the CU Meter.

2. Handling invalid CU requests: Transactions will fail if they request:
   - More than MAX_COMPUTE_UNIT_LIMIT per transaction, or
   - Less than the sum of all included builtin instructions'
     DEFAULT_COMPUTE_UNITS

## Alternatives Considered

None

## Impact

None

## Security Considerations

Both Agave and FD clients should implement this proposal to avoid breaking
consensus.

