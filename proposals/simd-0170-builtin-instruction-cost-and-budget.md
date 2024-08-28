---
simd: '0170'
title: Specifying CU Definitions for Built-ins
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

1. Built-in programs should consume a predefined number of CUs for each
   instruction.
2. Since built-ins can invoke other programs (CPI), they should allocate enough
   but granular CUs to ensure successful execution without over-allocation.

## Motivation

This proposal addresses two key issues related to CU allocation and consumption
for built-in programs while aiming to avoid adding unnecessary complexity.

1. **Accurate CU tracking without post-execution adjust-up**: Currently,
   built-in instructions deduct a fixed amount of CUs (DEFAULT_COMPUTE_UNITS)
from both the CU meter and block limits after execution. However, the CU meter
allocates a much larger value (DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT) to each
built-in instruction. This discrepancy creates imprecise CU tracking and may
require block producers to account for additional CUs after a transaction's
execution, potentially causing block limits to be exceeded.

   Furthermore, built-ins can conditionally CPI into other programs, introducing
variability in CU consumption. This unpredictability makes it difficult to
allocate CUs upfront solely based on CUs builtin consumes during execution, also
adds complexity to the tracking process.

2. **Preventing over-allocation of CUs**: Over-allocating CUs for built-ins
   reduces block density, lowers network throughput, and can degrade block
producer performance by causing repeated transaction retries. Avoiding excessive
CU allocation is critical for maximizing block efficiency and minimizing network
delays.

To resolve these issues, we propose statically defining the CU cost of built-in
instructions based on worst-case scenarios, including potential CPI calls. A
unified MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT will standardize CU allocation
for both block producers and the CU meter.

## Alternatives Considered

1. **Maintain current CU allocation with additional tracking logic**: One option
   is to keep the current DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT for built-ins
and add/modify tracking logic to the cost tracker. This would address the
discrepancy between CU allocation and consumption but increase system
complexity. The added logic could introduce corner cases and potential bugs,
raising the risk of issues in the transaction pipeline.

2. **Treat built-ins like regular instructions**: Another approach would be to
   allocate DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT for built-ins as for other
instructions. However, this fails to address over-allocation. Unlike regular
instructions, the execution of built-ins is more predictable and usually
requires significantly fewer CUs than BPF instructions. This approach would
allocate more CUs than necessary, undermining the goal of efficient CU usage.

3. **Declare both max and default CU values for built-ins**: A more precise
   approach would be to require built-ins to declare both a maximum CU
allocation (MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT) and a default CU value
for each instruction (DEFAULT_BUILTIN_INSTRUCTION_COMPUTE_UNITS). This would
allow fine-tuned CU allocation based on each instruction’s potential execution
path. However, this introduces new constraints for existing and future
built-ins, requiring updates to comply with these rules, which could
overcomplicate the design.

## Detailed Design

1. **Statically define CUs per instruction**: Assign a fixed CU consumption
   (DEFAULT_BUILTIN_INSTRUCTION_COMPUTE_UNITS) to each built-in instruction
rather than per built-in program.
2. **Set a static maximum CU allocation**: Propose a global limit of 5,000 CUs
   (MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT) to cover worst-case scenarios,
including CPI operations.
3. **Handling invalid CU requests**: Transactions will fail if they request:
   - More than MAX_COMPUTE_UNIT_LIMIT
   - Less than the sum of all included built-in instructions'
     MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT

If a transaction consists only of built-ins, no explicit CU request should be
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

