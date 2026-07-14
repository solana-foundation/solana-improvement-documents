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
when to fail a transaction. This SIMD proposes counting the top level 
instructions (executed or not) plus the executed CPIs. Consequently, 
transactions that exceed the instruction trace length will fail early.

## Motivation

The primary motivator for this change is ABIv2 
[SIMD-177](https://github.com/solana-foundation/solana-improvement-documents/pull/177).
The new memory layout exposes all top level instructions for every instruction 
in a transaction in pre-defined address spaces for exactly 64 instructions. 
Allowing a CPI in a transaction with 64 top level instructions, or, likewise, 
two CPIs in a transaction with 63 top level instructions, would cause runtime 
to overwrite a memory region already reserved for other purposes, due to 
the reservation of only 64 regions for instructions.

A second motivation would be slightly speeding up transactions that are going 
to fail regardless. This SIMD proposes failing offending transactions earlier, 
increasing block space for well formed ones.

## New Terminology

No new terminology introduced in this document.

## Detailed Design

When pushing the instruction to be processed onto the execution stack, the 
runtime must verify if the total number of top level instructions plus the 
number of CPIs does not exceed the maximum allowed instructions in the trace.

Below is a basic snippet of how this verification is recommended to look like:

```
if number_of_top_level_instructions_in_tx + number_of_cpis > 64 {
  return Err(InstructionError::InstructionTraceLengthExceeded);
}
```

It is worth pointing that the number of CPIs aforementioned already accounts 
for the CPI that is about to be pushed onto the execution stack, if we are not 
dealing with a top level instruction.

### Edge Cases

No edge case appears prominent enought to listed.

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

We have considered revising the address layout for ABIv2 
[SIMD-177](https://github.com/solana-foundation/solana-improvement-documents/pull/177)
to acommodate extra spaces for CPIs executed in transactions with 64 top level 
instructions or similar high numbers. We eliminated such alternative because 
these transactions would always fail. The address layout acommodation wouldn't 
benefit developers and would only increase complexity both for runtime and 
programs.

## Impact

There must be no difference to dapp developers, except for the number of CUs 
consumed by transactions that fail with instruction trace length exceeded.

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

