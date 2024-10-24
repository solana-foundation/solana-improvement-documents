---
simd: '0170'
title: Specifying CU Definitions for Builtins
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

1. Builtin programs should consume a predefined number of CUs for each
   instruction.
2. Since builtins can invoke other programs (CPI), they should allocate enough
   but granular CUs to ensure successful execution without over-allocation.

## Motivation

This proposal addresses two key issues related to CU allocation and consumption
for builtin programs while aiming to avoid adding unnecessary complexity.

1. **Accurate CU tracking without post-execution adjust-up**: Currently,
   builtin instructions deduct a fixed amount of CUs (DEFAULT_COMPUTE_UNITS)
from both the CU meter and block limits after execution. However, the CU meter
allocates a much larger value (DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT) to each
builtin instruction. This discrepancy creates imprecise CU tracking and may
require block producers to account for additional CUs after a transaction's
execution, potentially causing block limits to be exceeded.

   Furthermore, builtins can conditionally CPI into other programs, introducing
variability in CU consumption. This unpredictability makes it difficult to
allocate CUs upfront solely based on CUs builtin consumes during execution, also
adds complexity to the tracking process.

2. **Preventing over-allocation of CUs**: Over-allocating CUs for builtins
   reduces block density, lowers network throughput, and can degrade block
producer performance by causing repeated transaction retries. Avoiding excessive
CU allocation is critical for maximizing block efficiency and minimizing network
delays.

To resolve these issues, we propose statically defining the CU cost of builtin
instructions based on worst-case scenarios, including potential CPI calls. A
unified MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT will standardize CU allocation
for both block producers and the CU meter.

## Alternatives Considered

1. **Maintain current CU allocation with additional tracking logic**: One option
   is to keep the current DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT for builtins
and add/modify tracking logic to the cost tracker. This would address the
discrepancy between CU allocation and consumption but increase system
complexity. The added logic could introduce corner cases and potential bugs,
raising the risk of issues in the transaction pipeline.

2. **Treat builtins like regular instructions**: Another approach would be to
   allocate DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT for builtins as for other
instructions. However, this fails to address over-allocation. Unlike regular
instructions, the execution of builtins is more predictable and usually
requires significantly fewer CUs than BPF instructions. This approach would
allocate more CUs than necessary, undermining the goal of efficient CU usage.

3. **Declare both max and default CU values for builtins**: A more precise
   approach would be to require builtins to declare both a maximum CU
allocation (MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT) and a default CU value
for each instruction (DEFAULT_BUILTIN_INSTRUCTION_COMPUTE_UNITS). This would
allow fine-tuned CU allocation based on each instruction’s potential execution
path. However, this introduces new constraints for existing and future
builtins, requiring updates to comply with these rules, which could
overcomplicate the design.

## Detailed Design

1. **Statically define CUs per instruction**: Assign a fixed CU consumption
   (DEFAULT_BUILTIN_INSTRUCTION_COMPUTE_UNITS) to each builtin instruction
rather than per builtin program.
2. **Set a static maximum CU allocation**: Propose a global limit of 5,000 CUs
   (MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT) to cover worst-case scenarios,
including CPI operations.
3. **Handling invalid CU requests**: Transactions will fail if they request:
   - More than MAX_COMPUTE_UNIT_LIMIT
   - Less than the sum of all included builtin instructions'
     MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT

If a transaction consists only of builtins, no explicit CU request should be
required. If a CU request is made, the requested limit will override the max
allocation in #2.

## Impact

Users who previously relied on including builtin instructions, instead of
explicitly setting compute-unit limits, to allocate budget for their
transactions may experience an increase in transaction failures. To avoid this,
users are encouraged to use set_compute_unit_limit to explicitly request the
necessary budget for their transactions.

## Security Considerations

Both Agave and FD clients should implement this proposal to avoid breaking
consensus.

