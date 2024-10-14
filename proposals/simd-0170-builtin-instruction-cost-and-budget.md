---
simd: '0170'
title: Allocate precise builtin instructions budget
authors:
  - Tao Zhu (Anza)
category: Standard
type: Core
status: Draft
created: 2024-08-26
feature: https://github.com/anza-xyz/agave/issues/2562
supersedes: 
superseded-by:
extends:
---

## Summary

Builtin instructions always consume a statically defined amount of compute
units (CUs). Therefore, they should allocate the exact same amount of compute
budget and count the same amount toward the block limit during the banking
stage.

## Motivation

Builtin instructions in the SVM consume their static DEFAULT_COMPUTE_UNITS from
the compute meter during execution. These DEFAULT_COMPUTE_UNITS are also
counted against block limits during the banking stage. However, historically,
builtin instructions have been allocated DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT
units of compute budget. This discrepancy between the allowed consumption and
the actual usage tracked for block limits has led to several issues that need
to be addressed.

Allocating DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT instead of the actual
DEFAULT_COMPUTE_UNITS for builtin instructions distorts the tracking of
transaction compute budgets. This can result in more expensive instructions
being executed when they should fail due to exceeding the budget. Consequently,
the cost tracker may need to account for additional compute units toward block
limits after transaction execution, potentially producing blocks that exceed
those limits.

Furthermore, maintaining consistency in transaction costs between the banking
stage and SVM would simplify the code logic and make reasoning more
straightforward.

## Alternatives Considered

- One possible alternative approach would be to maintain the current allocation
of compute budget for builtin instructions but add logic to the cost tracker
to account for the discrepancy during tracking. However, this would add
complexity and could introduce additional corner cases, potentially leading to
more issues.

- Another alternative would be to treat builtin instructions the same as other
instructions by allocating DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT units for them
as well. However, this approach has concerns. DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT
is a very conservative estimate, currently set at 200,000 CUs per instruction.
Estimating all builtin instructions, including votes and transfers, would cause
the banking stage to significantly over-reserve block space during block
production, potentially leading to under-packed blocks. Additionally, if it's
known that builtin instructions will consume a fixed amount of CUs, it doesn't
make sense to estimate them with a generic DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT.

## New Terminology

None

## Detailed Design

The following changes are proposed to ensure consistent CU allocation and
consumption for builtin instructions: transactions containing only builtin
instructions should not need to explicitly request compute-unit-limits, and
these transactions should execute without any cost adjustment. If a transaction
explicitly sets a compute-unit-limit, the requested limit will be applied
accordingly.

1. Builtin instruction shall exposes two static CU measures:
   - EXECUTE_COMPUTE_UNITS, which represents the fixed amount of CUs to consume
     from CU meter for builtin instruction;
   - ALLOCATE_COMPUTE_UNITS, which represents amount CUs to allocate for the
     instruction in worse-case scenario; it accounts for any variation, such as
the instruction calling (CPI-ing) other instructions based on input data or
account stats. It must be equal or greater than the EXECUTE_COMPUTE_UNITS.

2. Explicitly requested Compute Units shall be fully respected; Requested CUs
   should be the only measure used everywhere CU is concerned, from block
production, CU metering during execution and elsewhere.

3. Transaction requesting invalid Compute Units shall fail; where "invalid" is
   defined as:
   - greater than `MAX_COMPUTE_UNIT_LIMIT`, or
   - lesser than sum of all included builtin instructions' ALLOCATE_COMPUTE_UNITS;

## Impact

Users who previously relied on including builtin instructions, instead of
explicitly setting compute-unit limits, to allocate budget for their
transactions may experience an increase in transaction failures. To avoid this,
users are encouraged to use set_compute_unit_limit to explicitly request the
necessary budget for their transactions.

## Security Considerations

Both Agave and FD clients should implement this proposal to avoid breaking
consensus.

