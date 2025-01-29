---
simd: '0170'
title: Reserve minimal CUs for builtins
authors:
  - Tao Zhu (Anza)
category: Standard
type: Core
status: Implemented
created: 2024-08-26
feature: C9oAhLxDBm3ssWtJx1yBGzPY55r2rArHmN1pbQn6HogH (https://github.com/anza-xyz/agave/issues/2562)
supersedes: 
superseded-by:
extends:
---

## Summary

When transactions do not request a specific compute unit limit, the Solana
runtime conservatively allocates 200,000 compute units for each tx-level
instruction (excluding compute budget instructions) in the transaction's
compute budget. Since builtin program instructions consume far less than 200k
compute units, the runtime will be modified to only allocate 3,000 compute units
in the transaction compute budget for each of these instructions.

## Motivation

When allocating the transaction compute budget for transactions which don't
specify a specific compute unit limit, the Solana runtime over-allocates the
number of compute units needed for executing builtin programs which have a
small (mostly) static execution compute unit cost. The runtime will only release
the over-allocated compute units from the block cost tracker when transaction
execution finishes and the actual consume compute units is known.

Over-allocating CUs for builtin program instructions reduces block density,
lowers network throughput, and can degrade block producer performance by causing
repeated transaction retries (due to repeatedly over allocating cu's and then
releasing them after execution). Avoiding excessive CU allocation is critical
for maximizing block efficiency and minimizing network delays.

## New Terminology

None

## Detailed Design

Establish a static maximum allocation of 3,000 CU's for builtin programs denoted
as `MAX_BUILTIN_ALLOCATION_COMPUTE_UNIT_LIMIT`, which is larger than all actual
compute unit costs of builtin programs as well as the potential Cross-Program
Invocations (CPIs) they do. This uniform limit is applied to each builtin
program instruction and is used to configure both CU meter allocations and block
producer CU limits reservation for all builtin instructions.

The static list of builtin program id's that will have 3,000 compute units
allocated are listed below, note that when builtins are migrated to sBPF
programs, they MUST be removed from this list and have the default 200k
compute units allocated instead.

- Stake11111111111111111111111111111111111111
- Config1111111111111111111111111111111111111
- Vote111111111111111111111111111111111111111
- 11111111111111111111111111111111
- ComputeBudget111111111111111111111111111111
- AddressLookupTab1e1111111111111111111111111
- BPFLoaderUpgradeab1e11111111111111111111111
- BPFLoader1111111111111111111111111111111111
- BPFLoader2111111111111111111111111111111111
- LoaderV411111111111111111111111111111111111
- KeccakSecp256k11111111111111111111111111111
- Ed25519SigVerify111111111111111111111111111

Note that there are a few builtin programs not in the list including the
upcoming zk programs as well as the feature program.

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

1. Do nothing and continue allocating 200k compute unit's for all non-compute
budget instructions. However, this fails to address over-allocation. Unlike
regular instructions, the execution of builtins is more predictable and usually
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

