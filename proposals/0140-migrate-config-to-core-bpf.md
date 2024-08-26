---
simd: '0140'
title: Migrate Config to Core BPF
authors:
  - Joe Caulfield - Anza Technology
category: Standard
type: Core
status: Draft
created: 2024-04-02
feature: [2Fr57nzzkLYXW695UdDxDeR5fhnZWSttZeZYemrnpGFV](https://github.com/anza-xyz/agave/issues/1378)
development:
  - Anza - [Implemented](https://github.com/anza-xyz/agave/pull/1379)
  - Firedancer - Not started
---

## Summary

Migrate the Config program to Core BPF.

## Motivation

BPF programs offer less complexity than native programs for other clients, such
as Firedancer, since developers will no longer have to keep up with program
changes in their runtime implementations. Instead, the program can just be
updated once.

In this spirit, the Config program should be migrated to Core BPF.

## Alternatives Considered

The Config program could instead remain a builtin program. This would mean each
validator client implementation would have to build and maintain this program
alongside their runtime, including any future changes.

## New Terminology

N/A.

## Detailed Design

The Config program will be reimplemented in order to be compiled to BPF and
executed by the BPF loader.

The reimplemented program's ABI will exactly match that of the original.

The reimplemented program's functionality will exactly match that of the
original, differing only in compute usage. Instead it will have dynamic compute
usage based on the VM's compute unit meter.

The program will be migrated to Core BPF using the procedure outlined in
[SIMD 0088](./0088-enable-core-bpf-programs.md).

The program will have no upgrade authority. If changes are required, for
essential network operations, the program will be updated using feature-gates.

## Impact

Validator client teams are no longer required to implement and maintain the
Config program within their runtime.

All validator client teams can work to maintain the single Config program
together.

## Security Considerations

The program's reimplementation poses no new security considerations compared to
the original builtin version.

The greatest security concern is a mistake in the reimplementation.

## Backwards Compatibility

The Core BPF implementation is 100% backwards compatible with the original
builtin implementation.

