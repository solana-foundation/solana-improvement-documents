---
simd: '0170'
title: Reserve minimal CUs for builtins
authors:
  - Tao Zhu (Anza)
category: Standard
type: Core
status: Review
created: 2024-08-26
feature: https://github.com/anza-xyz/agave/issues/2562
supersedes: 
superseded-by:
extends:
---

## Summary

If a transaction doesn't request a compute budget limit, then for each builtin
program instruction allocate 3,000 compute units rather than 200,000.

## Motivation

This proposal addresses two key issues related to CU allocation and consumption
for builtin programs while aiming to avoid adding unnecessary complexity.

1. Accurate CU tracking without post-execution adjust-up: Currently,
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

2. Preventing over-allocation of CUs: Over-allocating CUs for builtins
   reduces block density, lowers network throughput, and can degrade block
producer performance by causing repeated transaction retries. Avoiding excessive
CU allocation is critical for maximizing block efficiency and minimizing network
delays.

To resolve these issues, we propose statically defining the CU cost of builtin
instructions based on worst-case scenarios, including potential CPI calls. A
unified MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT will standardize CU allocation
for both block producers and the CU meter.

## New Terminology

None

## Detailed Design

Establish a static maximum CU allocation: Define a global compute unit limit
of 3,000 CUs, denoted as MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT, to
accommodate worst-case execution scenarios, including potential Cross-Program
Invocations (CPIs). This uniform limit is applied to each builtin program
instruction and is used to configure both CU meter allocations and block
producer CU limits reservation for all builtin instructions.

The 3,000 CU threshold is based on analysis of current built-in programs and
reflects the unlikely need for additional complexity. For instance, the
`AddressLookupTable`’s `CreateLookupTable` performs the highest number of CPI
calls (up to three invocations of the System program), resulting in a maximum
CU demand of 1,200 CUs (750 + 3 × 150). Similarly, `UpgradeableLoader`’s
`ExtendProgram`, which may invoke the System program once, requires up to
2,520 CUs (2,370 + 150), representing the most resource-intensive operation
among current built-ins. The proposed 3,000 CU limit slightly exceeds this
requirement, allowing for controlled flexibility without an excessive margin.

If a transaction consists only of builtins, no explicit CU request should be
required. If a CU request is made, the requested limit will override the max
allocation in #2.

## Alternatives Considered

1. Treat builtins like regular instructions: Another approach would be to
   allocate DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT for builtins as for other
instructions. However, this fails to address over-allocation. Unlike regular
instructions, the execution of builtins is more predictable and usually
requires significantly fewer CUs than BPF instructions. This approach would
allocate more CUs than necessary, undermining the goal of efficient CU usage.

2. Declare both max and default CU values for builtins: A more precise
   approach would be to require builtins to declare both a maximum CU
allocation (MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT) and a default CU value
for each instruction (DEFAULT_BUILTIN_INSTRUCTION_COMPUTE_UNITS). This would
allow fine-tuned CU allocation based on each instruction’s potential execution
path. However, this introduces new constraints for existing and future
builtins, requiring updates to comply with these rules, which could
overcomplicate the design.

## Impact

Users who previously relied on including builtin instructions, instead of
explicitly setting compute-unit limits, to allocate budget for their
transactions may experience an increase in transaction failures. To avoid this,
users are encouraged to use set_compute_unit_limit to explicitly request the
necessary budget for their transactions.

If those impacted users have issues fitting in the set compute unit limit
instruction into their transactions due to tx data size limits, they can also
migrate to using address lookup tables to fit in the compute budget instruction
call.

## Security Considerations

Both Agave and FD clients should implement this proposal to avoid breaking
consensus.

