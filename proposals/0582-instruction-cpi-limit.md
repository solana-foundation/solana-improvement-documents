---
simd: '0582'
title: Early detection of instruction trace overflow
authors:
  - Lucas Steuernagel
category: Standard
type: Core
status: Draft
created: 2026-07-13
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

The protocol only allows a transaction to execute 64 instructions, accounting 
for both top level and CPIs. Runtime counts executed instructions to determine 
when to fail a transaction. This SIMD proposes counting all top level 
instructions (executed or not) plus the executed CPIs, as the runtime attempts 
to add each instruction frame. Consequently, transactions that exceed the 
instruction trace length will fail early.

## Motivation

The primary motivation is slightly speeding up transactions that are going 
to fail regardless. This SIMD proposes failing offending transactions earlier, 
increasing block space for well formed ones.

The secondary motivator for this change is ABIv2 
[SIMD-177](https://github.com/solana-foundation/solana-improvement-documents/pull/177).
The new memory layout exposes all top level instructions for every instruction 
in a transaction in pre-defined address spaces for exactly 64 instructions. 
Allowing a CPI in a transaction with 64 top level instructions, or, likewise, 
two CPIs in a transaction with 63 top level instructions, would cause runtime 
to overwrite a memory region already reserved for other purposes, due to 
the reservation of only 64 regions for instructions.

## New Terminology

No new terminology introduced in this document.

## Detailed Design

Currently, the total number of instructions (top-level + CPI) is checked 
against the limit as each instruction is executed. Consequently, a transaction 
with 64 top-level instructions, whose first instruction performs a CPI, will 
still have 62 out of the remaining 63 instructions executed before 
exceeding the limit.

In the new design, when pushing the instruction to be processed onto the 
execution stack, the runtime must verify if the total number of top level 
instructions plus the number of CPIs does not exceed the maximum allowed 
instructions in the trace.

Below is a basic snippet of how this verification is recommended to look like:

```
if number_of_top_level_instructions_in_tx + number_of_cpis > 64 {
  return Err(InstructionError::InstructionTraceLengthExceeded);
}
```

It is worth pointing that the number of CPIs aforementioned already accounts 
for the CPI that is about to be pushed onto the execution stack, if we are not 
dealing with a top level instruction. The number of CPIs is tallied whenever a 
CPI is invoked, and can't be tracked ahead of time.

### Edge Cases

Transactions containing exactly 64 top-level instructions will fail as soon as 
the first CPI is pushed onto the execution stack, instead of executing all 64 
instructions in the budget provided by the protocol. Likewise, any transaction 
with `64-X` instructions, will fail as soon as `X+1` CPIs are pushed onto the 
stack.

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                              |
|---------------------------------|-------------------------------------|
| Transaction Execution (Runtime) | A condition is changing place       |
| Virtual Machine                 | None                                |
| Block Packing                   | None                                |
| Consensus                       | None                                |
| Gossip                          | None                                |
| Turbine                         | None                                |
| Snapshots                       | None                                |
| On-Chain Core BPF Programs      | None                                |
| Other (please describe)         | None                                |

## Alternatives Considered

None.

## Impact

There is a potential impact on the error order. Transactions that would 
exceed the instruction trace length, but are failing, for instance, because
of an error in a CPI before reaching the instruction limit, will now fail 
without such an error. As a consequence, developers may see different logs 
in transactions. In addition, CU consumption will change for transactions 
now halt earlier.

Validators should see more block space as offending transactions will fail 
earlier.

## Security Considerations

This proposal suggests a change basic enough to have a low security risk.

## Conformance

Once implemented, we will generate new fixtures for the correct conformance 
testing between validator implementation. We are going to use the SVM 
conformance module for transactions implemented in Agave PR 
[#13211](https://github.com/anza-xyz/agave/pull/13211), allowing for 
differential fuzzing in an uniform interface.

