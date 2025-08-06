---
simd: '0182'
title: Consume requested CUs for sBPF failures
authors:
  - Tao Zhu (Anza)
category: Standard
type: Core
status: Implemented
created: 2024-10-03
feature: B7H2caeia4ZFcpE3QcgMqbiWiBtWrdBRBSJ1DY6Ktxbq (https://github.com/anza-xyz/agave/issues/3993)
supersedes:
superseded-by:
extends:
---

## Summary

Adjusting how CU consumption is measured based on the conditions of transaction
execution: successful completion will consume actual CUs, but certain irregular
failures in the sBPF VM will result in the transaction automatically consuming
all requested CUs.

## Motivation

In the Solana protocol, tracking transaction Compute Unit (CU) consumption is a
critical aspect of maintaining consensus. Block costs are part of this
consensus, meaning that all clients must agree on the execution cost of each
transaction, including those that error out during execution. Ensuring
consistency in CU tracking across clients is essential for maintaining protocol
integrity.

To improve performance, Solana programs are often compiled with a JIT that works
at the level of Basic Blocks — linear sequences of sBPF instructions with a
single entry and exit point, and no loops or branches. Basic Blocks allow for
efficient execution by reducing the overhead associated with tracking CU
consumption for each individual sBPF instruction.

Other than in less common situations discussed below, the total CU
consumption for a Basic Block is deterministic and CU accounting can be
done once per basic block instead of at each instruction. A transaction
completing successfully or with most errors implies that execution exited each
basic block at its single exit point, and thus that the total CU consumption of
the execution is equal to the sum of the CU cost of each Basic Block executed.

However, when an exception is thrown during the execution of a Basic Block
(e.g., a null memory dereference or other faults), determining the exact number
of CUs consumed up to the point of failure requires additional effort. For
instance, the Agave client implements a mechanism that tracks the Instruction
Pointer (IP) or Program Counter (PC) to backtrack and estimate the CUs consumed
when an exception occurs. More details on this mechanism can be found
[here](https://github.com/solana-labs/rbpf/blob/57139e9e1fca4f01155f7d99bc55cdcc25b0bc04/src/jit.rs#L267).

While this approach is effective, it introduces additional work and complexity.
These mechanisms are often implementation-specific, and requiring all clients to
track the exact number of executed sBPF instructions for consensus is costly and
unnecessary. Such precision is not essential for protocol-level consensus,
especially since these cases are infrequent.

Instead of mandating implementation-specific work to handle exceptions, we
propose the following clarification in the protocol:

- For successful execution of a Basic Block (i.e., the block exits at the last
 sBPF instruction), the deterministic CU cost of the block will be charged to
the transaction’s CU meter. This ensures that CU consumption for successful
transactions is accurately accounted for.
- In the event of irregular failure, where execution aborts from the middle of
basic block, the requested CUs for the transaction will be charged to the CU
meter. This allows for a simple and efficient fallback mechanism that avoids the
need for tracking the exact number of executed instructions up to the point of
failure.

By adopting this approach, the protocol avoids the overhead of requiring precise
instruction-level CU tracking for transactions that fail. Instead, the requested
CU limit of the transaction will be used, simplifying the handling of
irregularly failed transactions while still maintaining consensus.

## Alternatives Considered

None

## New Terminology

- [Basic Block](https://en.wikipedia.org/wiki/Basic_block): In the context of
  JIT execution and BPF processing, a Basic Block is a sequence of BPF
instructions that forms a single, linear flow of control with no loops or
conditional branches except for the entry and exit points. It represents a
segment of code where execution starts at the first instruction and proceeds
sequentially through to the last instruction without deviation. The Basic Block
is characterized by its predictable execution path, allowing for efficient
budget checks and optimizations, as its Compute Unit (CU) cost can be determined
before execution and verified at the end of the block.

- Irregular transaction failure: A rare case that a Transaction execution aborts
in the middle of executing basic block, results in consuming all requested CUs.

## Detailed Design

If VM execution returns any error except `SyscallError`, transaction's CU meter
should be fully depleted, in another words, all requested CUs are consumed;
otherwise consumes the actual executed CUs.

## Impact

None

## Security Considerations

None
