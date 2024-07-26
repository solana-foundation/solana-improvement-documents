---
simd: '0128'
title: Migrate Address Lookup Table to Core BPF
authors:
  - Joe Caulfield - Anza Technology
category: Standard
type: Core
status: Draft
created: 2024-03-13
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Migrate the Address Lookup Table program to Core BPF.

## Motivation

BPF programs offer less complexity than native programs for other clients, such
as Firedancer, since developers will no longer have to keep up with program
changes in their runtime implementations. Instead, the program can just be
updated once.

In this spirit, Address Lookup Table should be migrated to Core BPF.

## Alternatives Considered

The Address Lookup Table program could instead remain a builtin program. This
would mean each validator client implementation would have to build and maintain
this program alongside their runtime, including any future changes.

## New Terminology

N/A.

## Detailed Design

The Address Lookup Table program is reimplemented in order to be compiled
to BPF and executed by the BPF loader.

The reimplemented program's ABI exactly matches that of the original.

The reimplemented program's functionality exactly matches that of the
original, differing only in compute usage. Instead it has dynamic compute
usage based on the VM's compute unit meter.

The reimplemented program can be found here:
https://github.com/solana-program/address-lookup-table.

The program shall be migrated to Core BPF using the procedure outlined in
[SIMD 0088](./0088-enable-core-bpf-programs.md).

The program has no upgrade authority. If changes are required, for
essential network operations, the program will be updated using feature-gates.

## Impact

Validator client teams are no longer required to implement and maintain the
Address Lookup Table program within their runtime.

All validator client teams can work to maintain the single Address Lookup Table
program together.

## Security Considerations

The program's reimplementation poses no new security considerations compared to
the original builtin version.

The greatest security concern is a mistake in the reimplementation.

## Backwards Compatibility

The Core BPF implementation is 100% backwards compatible with the original
builtin implementation.

