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

The following changes are proposed to ensure consistent CU allocation and
consumption for builtin instructions: transactions containing only builtin
instructions should not need to explicitly request compute-unit-limits, and
these transactions should execute without any cost adjustment. If a transaction
explicitly sets a compute-unit-limit, the requested limit will be applied
accordingly.

1. Allocate Compute Budget per Builtin Instruction:

  When set_compute_unit_limit is not explicitly requested, the compute budget
  should always allocate the maximum number of compute units (MAX_COMPUTE_UNIT)
  as declared by each individual instruction, including compute-budget
  instructions.

  Currently, when no set_compute_unit_limit is used, all instructions are
  allocated a DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT, which is capped by the
  transaction’s MAX_COMPUTE_UNIT_LIMIT. However, historically, no CUs have been
  allocated for compute-budget instructions, which can lead to over-packed
  blocks in certain cases.

  Example Scenarios:

  [Example 1](https://github.com/anza-xyz/agave/pull/2746/files#diff-c6c8658338536afbf59d65e9f66b71460e7403119ca76e51dc9125e1719f4f52R13403-R13429):
  A transaction contains both a Transfer instruction and an expensive non-
  builtin instruction. If the non-builtin instruction requires more compute
  units than the DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT, over-budgeting for the
  Transfer instruction allows the expensive instruction to complete execution,
  forcing an upward adjustment of the compute units used.

  [Example 2](https://github.com/anza-xyz/agave/pull/2746/files#diff-c6c8658338536afbf59d65e9f66b71460e7403119ca76e51dc9125e1719f4f52R13431-R13451):
  A builtin instruction that calls (CPI) other instructions may not reserve
  enough CUs. Upon successful execution, this under-reservation forces an
  upward adjustment.

### Detailed Changes:

  1.1 Changes to Builtin Programs:

    - Each builtin program will now expose the DEFAULT_COMPUTE_UNIT for each of
    its instructions (similar to how ZK programs do it).
    - Each instruction will also expose its MAX_COMPUTE_UNIT, which represents
    the worst-case scenario. This MAX_COMPUTE_UNIT accounts for any variation,
    such as the instruction calling (CPI-ing) other instructions based on input
    data or account states. It must be equal to or greater than the
    DEFAULT_COMPUTE_UNIT.
    - This makes builtin programs more transparent about their compute usage.
    During execution, the runtime will use the DEFAULT_COMPUTE_UNIT to track
    actual compute usage, while MAX_COMPUTE_UNIT will be used to allocate the
    compute budget.

  1.2 Changes to Builtin-Default-Costs Crate:

    Instead of the current dictionary of [builtin program, DEFAULT_COMPUTE_UNIT],
    a new dictionary will be created with [instruction, MAX_COMPUTE_UNIT].
    This will allow the system to accurately calculate compute allocation for
    each instruction, factoring in its potential CPIs.

  1.3 Call-Site Implementation:

    - Instruction Type Lookup: At the call-site (e.g., in the compute budget or
    cost model), the type of builtin instruction will be determined.
    - If the instruction type cannot be identified, a small number of CUs will
    be allocated to account for basic work done on the transaction, such as
    deserialization or instruction type lookup.
    - If the instruction type is determined, the system will allocate the
    MAX_COMPUTE_UNIT for that instruction.
    - The transaction’s program_id_index will only be checked once during this
    process, and the result will be cached to prevent redundant lookups for
    efficiency.


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
users are encouraged to use set_compute_unit_limit to explicitly request the
necessary budget for their transactions.

## Security Considerations

Both Agave and FD clients should implement this proposal to avoid breaking
consensus.

