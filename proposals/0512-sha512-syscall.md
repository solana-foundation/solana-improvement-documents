---
simd: '0512'
title: Sha512 Syscall
authors:
  - Dean Little (Blueshift)
category: Standard
type: Core
status: Idea
created: 2026-04-03
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Introduce a `sol_sha512` syscall with an identical interface to `sol_sha256`,
producing SHA-512 hashes and outputting the 64-byte result.

## Motivation

SHA-512 is a core primitive of Ed25519 signature verification and is already
present in both the Agave and Firedancer validator clients as an internal
dependency. However, it is not currently exposed to on-chain programs as a
syscall.

Exposing `sol_sha512` directly enables programs to perform SHA-512 hashing
without resorting to an on-chain software implementation, which is both
expensive in compute units and unnecessary given the function already exists
in every validator.

## New Terminology

N/A

## Detailed Design

### Syscall Signature

The syscall follows the same interface as `sol_sha256`, `sol_keccak256`,
and `sol_blake3`. See here for more details:

https://docs.rs/solana-sha256-hasher/3.1.0/src/solana_sha256_hasher/lib.rs.html#62-75

The syscall computes SHA-512 over the provided byte slices as if they were
a single contiguous input, writing the 64-byte digest to `result`.

The syscall aborts the virtual machine if any of these conditions are true:

- Not all bytes in `[bytes, bytes + bytes_len * sizeof(SolBytes))` are
  readable.
- Not all bytes in each slice `[bytes[i].addr, bytes[i].addr + bytes[i].len)`
  are readable.
- Not all bytes in `[result, result + 64)` are writable.
- `bytes_len` exceeds the configured maximum number of slices. The current
  default is 20,000, as per `SVMTransactionExecutionBudget`. See here:
https://github.com/anza-xyz/agave/blob/289aa4ea46889a1535962b727c0656d4d25527dc/program-runtime/src/execution_budget.rs#L82

### Compute Unit Usage

Compute costs follow the same model and parameters as `sol_sha256`. 

See here for more details: 
https://github.com/anza-xyz/agave/blob/master/syscalls/src/lib.rs#L168-L169

## Alternatives Considered

### BPF Implementation

Programs can implement SHA-512 in BPF today, but at a higher CU cost.
A single SHA-512 hash of a short message consumes thousands of CUs in
software versus fewer than 100 via syscall.

### Status Quo

Continue without exposing SHA-512. Programs requiring SHA-512 (e.g., for
Ed25519-adjacent verification logic) remain unable to access a primitive
that both validators already have linked.

## Impact

Programs gain access to SHA-512 hashing at syscall cost.

## Security Considerations

The security surface is identical to the existing hash syscalls. The same
input validation and CU metering apply. SHA-512 itself is a well-studied,
standardized hash function (FIPS 180-4).

## Backwards Compatibility

This is an additive change gated behind a feature flag. Programs that do not
invoke `sol_sha512` are unaffected. Existing syscalls are unchanged.
