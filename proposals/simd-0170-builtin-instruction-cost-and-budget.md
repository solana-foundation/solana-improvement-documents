---
simd: '0170'
title: Allocate percise builtin instructions budget
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
the compute budget during execution. These DEFAULT_COMPUTE_UNITS are also
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

To ensure consistency, the following three changes are proposed:

1. Allocate Static Compute Budget for Builtin Instructions:
   When the compute-unit-limit is not explicitly requested, the compute budget
should always allocate the statically defined DEFAULT_COMPUTE_UNITS for builtin
instructions, including compute-budget instructions.

   Currently, when set_compute_unit_limit is not used, all instructions are
allocated DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT, capped by the transaction's
MAX_COMPUTE_UNIT_LIMIT. However, compute-budget instructions historically
haven't had any units allocated. This can lead to over-packed blocks in
certain scenarios.

   [Example 1](https://github.com/anza-xyz/agave/pull/2746/files#diff-c6c8658338536afbf59d65e9f66b71460e7403119ca76e51dc9125e1719f4f52R13403-R13429):
   A transaction consists of a Transfer and an expensive non-builtin instruction.
If the non-builtin instruction requires more than DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT
CUs, the over-budgeting for Transfer allows the expensive instruction to
execute to completion, forcing an upward adjustment.

   [Example 2](https://github.com/anza-xyz/agave/pull/2746/files#diff-c6c8658338536afbf59d65e9f66b71460e7403119ca76e51dc9125e1719f4f52R13431-R13455):
   A builtin instruction that might call other instructions (CPI) would fail
without explicitly requesting more CUs. However, this isn't currently happening.

2. Respect Explicit Compute Unit Requests During Block Production:
   When a compute-unit-limit is explicitly requested, always use it to reserve
block space during block production, even if there are no user-space instructions.

   [Example 3](https://github.com/anza-xyz/agave/pull/2746/files#diff-c6c8658338536afbf59d65e9f66b71460e7403119ca76e51dc9125e1719f4f52R13457-R13484)
   The cost model ignores explicitly requested CUs for transactions has all
buitin instructions, resulting in an upward adjustment instead of the usual
downward adjustment.

3. Fail Transactions with Invalid Compute Unit Limits Early:
   If set_compute_unit_limit sets an invalid value, the transaction should fail
before being sent for execution.
   "invalid value" is defined as `> MAX_COMPUTE_UNIT_LIMIT || < Sum(builtin_instructions)`

   [Example 4](https://github.com/anza-xyz/agave/pull/2746/files#diff-c6c8658338536afbf59d65e9f66b71460e7403119ca76e51dc9125e1719f4f52R13344-R13373):
   If the explicitly requested CU limit is invalid, the transaction should
fail during sanitization, saving it from being sent to the SVM for execution.


*Note*: Users are encouraged to explicitly request a reasonable amount of
compute-unit-limits. Requesting more than needed not only increases the
prioritization fee the user pays but also lowers the transaction's priority.

## Impact

Users who previously relied on including builtin instructions, instead of
explicitly setting compute-unit limits, to allocate budget for their
transactions may experience an increase in transaction failures. To avoid this,
users are encouraged to use set_compute_unit_limits to explicitly request the
necessary budget for their transactions.

## Security Considerations

Both Agave and FD clients should implement this proposal to avoid breaking
consensus.

