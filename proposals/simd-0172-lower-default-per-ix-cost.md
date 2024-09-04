---
simd: '0172'
title: Reduce default CU per instruction to zero
authors:
  - Tao Zhu (Anza)
category: Standard
type: Core
status: Draft
created: 2024-08-30
feature: 
supersedes: 
superseded-by:
extends:
---

## Summary

The `DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT` was originally defined for
worst-case scenarios at 200,000 CUs per instruction. This proposal suggests
reducing it to zero per instruction.

## Motivation

1. Reducing Overestimation for Better Block Packing:
The current DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT is set at a conservative
200,000 CUs per instruction to account for worst-case scenarios. However, this
overestimation leads to excessive reservation of block resources, resulting in
underutilized blocks. By gradually reducing this limit, we can better align the
allocated compute units with actual usage, leading to more efficient block
packing. This change will optimize resource allocation, enabling the production
of more densely packed blocks and improving overall system performance.

2. Encouraging Accurate Compute Unit Requests:
The high default compute unit limit has allowed users to be less precise in
their compute unit requests, often resulting in inefficient resource usage.
By lowering the DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT, we encourage users to
explicitly request the amount of compute units needed for their transactions.
This shift towards more accurate compute unit requests will gradually foster a
more efficient and predictable compute unit allocation paradigm, benefiting
both developers and the network by ensuring that transactions are appropriately
budgeted and that resources are optimally utilized.

## Alternatives Considered

1. Lowering the Default to an Arbitrary Value:
One alternative is to reduce the DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT to an
arbitrary value, such as 20,000 CUs per instruction, based on current network
statistics. While this approach could address the overestimation issue in the
short term, it may not be sustainable as network conditions evolve. The chosen
value may become inadequate in the future, necessitating further adjustments.
Additionally, this approach does little to encourage users to explicitly
request the compute units needed for their transactions, meaning it fails to
address the second motivation of promoting more accurate compute unit requests.

2. Increasing Block Limits While Keeping Account Limits Unchanged:
Another alternative is to increase the overall block limits while keeping
account limits unchanged. This could superficially solve the overestimation
issue by allowing more transactions per block. However, this approach only
alleviates the problem temporarily and does not address the root cause. It also
does not promote the transition toward a more accurate compute unit requesting
paradigm, as users would still not be incentivized to precisely define their
compute unit needs.

## New Terminology

None

## Detailed Design

1. Target Value Reduction:
Set the target value for DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT to zero.

2. Scheduled Ramp-Down:
Implement a gradual reduction toward the target value by decreasing
DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT by 20,000 CUs per epoch. This phased
approach gives developers time to adjust to the new limits.

3. Final Removal:
Once the target value of zero is reached, remove
DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT entirely from the system.


### Note:

When reached to target default, cost tracking can be switched to ALWAYS use
requested CUs.

## Impact

1. Increased Block Density:
The proposal will lead to more densely packed blocks, increasing revenue for
block producers.

2. Developer Adjustments:
Developers will be more inclined to explicitly include set-compute-unit-limit
for their transactions, ensuring accurate compute unit allocation.

3. No Impact on Simple Transactions:
Simple transactions, such as transfers, and other builtin-only transactions
like voting, will remain unaffected. These transactions will continue to
function without the need for explicit set-compute-unit-limit settings,
especially after the implementation of SIMD-0170.

## Security Considerations

Both Agave and FD clients should implement this proposal to avoid breaking
consensus.


