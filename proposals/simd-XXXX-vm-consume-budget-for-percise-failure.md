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

Adjusting how CU consumption is measured based on the conditions of transaction
execution: successful completion will consume actual CUs, but certain irregular
failures will result in the transaction automatically consuming all requested
CUs.

## Motivation

### Background:

In the Solana protocol, tracking transaction Compute Unit (CU) consumption is a
critical aspect of maintaining consensus. Block costs are part of this
consensus, meaning that all clients must agree on the execution cost of each
transaction, including those that error out during execution. Ensuring
consistency in CU tracking across clients is essential for maintaining protocol
integrity.

### Proposed Change:

To improve performance, Solana programs are often compiled with a JIT that works
at the level of Basic Blocks — linear sequences of sBPF instructions with a
single entry and exit point, and no loops or branches. Basic Blocks allow for
efficient execution by reducing the overhead associated with tracking CU
consumption for each individual BPF instruction.

Other than in rare, exceptional situations discussed below, the total CU
consumption for a Basic Block is deterministic and, and CU accounting can be
done once per basic block instead of at each instruction.  A transaction
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
track the exact number of executed BPF instructions for consensus is costly and
unnecessary. Such precision is not essential for protocol-level consensus,
especially since these cases are rare.

### Clarified Protocol Behavior:

Instead of mandating implementation-specific work to handle exceptions, we
propose the following clarification in the protocol:

- For successful execution of a Basic Block (i.e., the block exits at the last
  BPF instruction), the deterministic CU cost of the block will be charged to
the transaction’s CU meter. This ensures that CU consumption for successful
transactions is accurately accounted for.
- In the event of an exception during Basic Block execution, where the block
  does not exit normally, the requested CUs for the transaction will be charged
to the CU meter. This allows for a simple and efficient fallback mechanism that
avoids the need for tracking the exact number of executed instructions up to the
point of failure.

By adopting this approach, the protocol avoids the overhead of requiring precise
instruction-level CU tracking for transactions that fail. Instead, the requested
CU limit of the transaction will be used, simplifying the handling of failed
transactions while still maintaining consensus.

### Conclusion:

This proposal enhances performance and simplifies CU tracking by formalizing the
use of Basic Blocks for efficient execution. It eliminates the need for costly,
implementation-specific work to track CU consumption during execution failures,
providing a clear and consistent approach to handling exceptions. This change
allows clients to maintain consensus without sacrificing performance, ensuring
that the protocol remains both efficient and robust.

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

Banking stage and Replay stage collect transaction execution results and CUs
consumed at
[here](https://github.com/anza-xyz/agave/blob/master/core/src/banking_stage/committer.rs#L99)
and
[here](https://github.com/anza-xyz/agave/blob/master/ledger/src/blockstore_processor.rs#L239),
respectively.

Where the `committed_tx.executed_units` are accumulated value of each
Instruction `compute_unit_consumed`, which essentially is the changes of
invoke_context's CU_meter.

We propose VM to deplete CU meter when irregular failure occurs during
execution, which is following errors in Agave:

```
EbpfError::DivideByZero
EbpfError::DivideOverflow
EbpfError::CallOutsideTextSegment
EbpfError::InvalidInstruction
EbpfError::InvalidVirtualAddress
```

And in Firedancer:

```
#define FD_VM_ERR_SIGSPLIT    ( -9) /* split multiword instruction (e.g. jump into the middle of a multiword instruction) */
#define FD_VM_ERR_SIGILL      (-12) /* illegal instruction (e.g. opcode is not valid) */
#define FD_VM_ERR_SIGSEGV     (-13) /* illegal memory address (e.g. read/write to an address not backed by any memory) */
#define FD_VM_ERR_SIGBUS      (-14) /* misaligned memory address (e.g. read/write to an address with inappropriate alignment) */
#define FD_VM_ERR_SIGRDONLY   (-15) /* illegal write (e.g. write to a read only address) */
#define FD_VM_ERR_SIGFPE      (-18) /* divide by zero */
```

In this way, detecting irregular failure is fully encapsulated within VMs, call
sites can continue work on Execution Results without change.

### Alternatives:

No changes to VM, instead at call sites, e.g. at Banking Stage and Replay Stage
to check execution results then determine how many CUs to consume, like this:

```
let execution_cu = match transaction.execution_results {
  irregualr_execution_failure => transaction.requested_cu,
  _ => committed_tx.executed_cu,
};
```

This alternative requires mapping VM error to runtime InstructionError, and
pushing irregualr failure detection upstream to call sites.

## Impact

None

## Security Considerations

One potential issue with using requested CUs in the case of failed transactions
is the risk of transactions with grossly large CU requests consuming an
excessive portion of the block's CU limit. This could effectively cause a
denial-of-service effect by preventing legitimate transactions from being
included in the block. To mitigate this risk, it is recommended that this
proposal be implemented after SIMD-172 is deployed, which removes the
possibility of accidentally requesting an excessively large number of CUs.

By ensuring that CU requests are reasonable and controlled, the risk of failed
transactions taking up disproportionate block space will be minimized, allowing
the proposed solution to work effectively without compromising block
utilization.
