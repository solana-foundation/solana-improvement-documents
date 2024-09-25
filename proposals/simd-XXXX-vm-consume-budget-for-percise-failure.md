---
simd: 'XXXX'
title: Conditional CU metering
authors:
  - Tao Zhu (Anza)
category: Standard
type: Core
status: Draft
created: 2024-MM-DD
feature: 
supersedes: 
superseded-by:
extends:
---

## Summary

Adjusting how CU consumption is measured based on the conditions of Basic Block execution: successful completion will charge actual CUs, or requested CU if exceptions during Basic Block execution.

## Motivation

### Background:

In the Solana protocol, tracking transaction Compute Unit (CU) consumption is a critical aspect of maintaining consensus. Block costs are part of this consensus, meaning that all clients must agree on the execution cost of each transaction, including those that error out during execution. Ensuring consistency in CU tracking across clients is essential for maintaining protocol integrity.

### Proposed Change:

To improve performance, Solana programs are often compiled into Basic Blocks — linear sequences of BPF instructions with a single entry and exit point, and no loops or branches. Basic Blocks allow for efficient execution by reducing the overhead associated with tracking CU consumption for each individual BPF instruction.

When a Basic Block is executed successfully (i.e., it exits at the final BPF instruction in the block), the total CU consumption is deterministic and can be calculated before execution. This ensures that CU accounting for successful transactions is accurate and predictable, enabling all clients to agree on the transaction’s execution cost.

However, when an exception is thrown during the execution of a Basic Block (e.g., a null memory dereference or other faults), determining the exact number of CUs consumed up to the point of failure requires additional effort. For instance, the Agave client implements a mechanism that tracks the Instruction Pointer (IP) or Program Counter (PC) to backtrack and estimate the CUs consumed when an exception occurs. More details on this mechanism can be found [here](https://github.com/solana-labs/rbpf/blob/57139e9e1fca4f01155f7d99bc55cdcc25b0bc04/src/jit.rs#L267).

While this approach is effective, it introduces additional work and complexity. These mechanisms are often implementation-specific, and requiring all clients to track the exact number of executed BPF instructions for consensus is costly and unnecessary. Such precision is not essential for protocol-level consensus.

### Clarified Protocol Behavior:

Instead of mandating implementation-specific work to handle exceptions, we propose the following clarification in the protocol:

- For successful execution of a Basic Block (i.e., the block exits at the last BPF instruction), the deterministic CU cost of the block will be charged to the transaction’s CU meter. This ensures that CU consumption for successful transactions is accurately accounted for.
- In the event of an exception during Basic Block execution, where the block does not exit normally, the requested CUs for the transaction will be charged to the CU meter. This allows for a simple and efficient fallback mechanism that avoids the need for tracking the exact number of executed instructions up to the point of failure.

By adopting this approach, the protocol avoids the overhead of requiring precise instruction-level CU tracking for transactions that fail. Instead, the requested CU limit of the transaction will be used, simplifying the handling of failed transactions while still maintaining consensus.

### Conclusion:

This proposal enhances performance and simplifies CU tracking by formalizing the use of Basic Blocks for efficient execution. It eliminates the need for costly, implementation-specific work to track CU consumption during execution failures, providing a clear and consistent approach to handling exceptions. This change allows clients to maintain consensus without sacrificing performance, ensuring that the protocol remains both efficient and robust.

## Alternatives Considered

None

## New Terminology

- [Basic Block](https://en.wikipedia.org/wiki/Basic_block):i In the context of JIT execution and BPF processing, a Basic Block is a sequence of BPF instructions that forms a single, linear flow of control with no loops or conditional branches except for the entry and exit points. It represents a segment of code where execution starts at the first instruction and proceeds sequentially through to the last instruction without deviation. The Basic Block is characterized by its predictable execution path, allowing for efficient budget checks and optimizations, as its Compute Unit (CU) cost can be determined before execution and verified at the end of the block.

## Detailed Design

At banking stage [here](https://github.com/anza-xyz/agave/blob/master/core/src/banking_stage/committer.rs#L99) and replay stage [here](https://github.com/anza-xyz/agave/blob/master/ledger/src/blockstore_processor.rs#L239) where Transaction's executed_units is checked, implement new logic:
```
let execution_cu = match transaction.execution_results {
    Ok(_) || Err(TransactionError::CustomError(_)) => committed_tx.executed_cu,
    _ => transaction.requested_cu,
};
... ...
```

## Impact

None

## Security Considerations

One potential issue with using requested CUs in the case of failed transactions is the risk of transactions with grossly large CU requests consuming an excessive portion of the block's CU limit. This could effectively cause a denial-of-service effect by preventing legitimate transactions from being included in the block. To mitigate this risk, it is recommended that this proposal be implemented after SIMD-172 is deployed, which removes the possibility of accidentally requesting an excessively large number of CUs.

By ensuring that CU requests are reasonable and controlled, the risk of failed transactions taking up disproportionate block space will be minimized, allowing the proposed solution to work effectively without compromising block utilization.
