---
simd: '0670'
title: ABIv1 invoke signed V2 syscall
authors:
  - febo (Anza)
category: Standard
type: Core
status: Review
created: 2026-09-25
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This proposal introduces a new invoke syscall with an improved API to handle
account information in CPI (cross-program invocation) calls, aiming to reduce
the memory overhead and simplify the process of passing account data between
programs and runtime.

## Motivation

Currently, configuring CPI parameters in ABIv1 programs is unnecessarily
cumbersome:

1. It requires passing two account slices: `&[AccountMeta]` and
   `&[AccountInfo]`.
2. It requires converting account information received as program input into a
   different layout, even though the runtime subsequently converts it into yet
   another layout.

Both requirements force programs to allocate and copy more memory than strictly
necessary. This overhead could be avoided without negatively affecting runtime
performance. An `AccountMeta` occupies `34` bytes, while an `AccountInfo`
occupies `56` bytes. Therefore, a program must manipulate `90` bytes per account
for each CPI. This often requires heap allocation because the total size can
exceed the maximum stack size of `4096` bytes, particularly given that a CPI can
accept up to `255` accounts.

## New Terminology

N/A

## Detailed Design

To address the first issue, we can eliminate one of the account slices by
combining the information contained in `AccountMeta` and `AccountInfo`. The
purpose of `AccountMeta` is to associate the writable and signer flags with an
account, while `AccountInfo` contains the actual account information.

The second issue can be mitigated by allowing programs to simply pass the index
of the account in the instruction rather than requiring them to copy the account
information into a different layout. The runtime already has access to the
account information, so it can look up the account by index and retrieve the
necessary information.

These two changes can be combined by defining a new type `CpiAccount` that
associates the account index with the writable/signer flags, reducing the
per-account memory requirement for a CPI from `90` bytes to `4` bytes.

```rust
/// Representation of an account used in a CPI.
#[repr(C)]
pub struct CpiAccount {
    /// Index of the account in the current instruction.
    pub account_index: u16,

    /// Indicates whether the account is writable.
    pub is_writable: u8,

    /// Indicates whether the account signed the instruction.
    pub is_signer: u8,
}
```

The program creates a new `CpiInstruction` containing a single slice of the
accounts expected by the invoked program:

```rust
/// Information about an instruction for a CPI.
#[repr(C)]
pub struct CpiInstruction {
    /// Address of the program to invoke.
    program_id: * const [u8; 32],

    /// Accounts expected by the program instruction.
    accounts: *const CpiAccount,

    /// Number of accounts expected by the program instruction.
    accounts_len: u64,

    /// Data expected by the program instruction.
    data: *const u8,

    /// Length of the data expected by the program instruction.
    data_len: u64,
}
```

Finally, the `CpiInstruction` is used as a parameter to a new `invoke_signed_v2`
syscall:

```rust
pub fn sol_invoke_signed_v2(
    instruction_addr: u64,
    signers_seeds_addr: u64,
    signers_seeds_len: u64,
);
```

These modifications eliminate only the type conversions performed on the program
side. The CPI logic in the runtime remains unchanged, apart from reading the
account information through the instruction account index during account
translation. In other words, this proposal does not change the runtime's CPI
logic, but it does change the way programs pass account information to the
runtime.

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                              |
|---------------------------------|-------------------------------------|
| Transaction Execution (Runtime) | None                                |
| Virtual Machine                 | New syscall                         |
| Block Packing                   | None                                |
| Consensus                       | None                                |
| Gossip                          | None                                |
| Turbine                         | None                                |
| Snapshots                       | None                                |
| On-Chain Core BPF Programs      | None                                |
| Other (please describe)         | None                                |

## Alternatives Considered

1. Continue without the new syscall. This would maintain the current overhead of
   CPIs, which is unnecessary and inefficient.

2. Continue to use the existing `AccountMeta`, but pass a slice of account
   pointers to the runtime instead of `AccountInfo`. This would reduce the
   memory requirement from `90` to `42` bytes (`34` + `8` bytes) per account.
   This does not offer as much reduction in memory overhead as the proposed
   solution, and also still requires a second slice of accounts to be passed to
   the runtime.

3. Instead of using the index of an account in the transaction, pass account
   pointers to the runtime. This reduces the memory requirement from `90` to
   `16` bytes per account, including `6` bytes of padding &mdash; the program
   itself copies only `10` bytes per account. While this reduces memory
   overhead, the current proposal is more efficient. In particular, using
   account indices allows the CPI account list to be constructed statically at
   compile time when the program’s account list is deterministic.

## Impact

The proposal will reduce the memory overhead of CPIs, which will improve
performance and reduce the likelihood of heap allocation. This will benefit
program developers by making it easier to write efficient programs.

Below is a benchmark showing reductions in CU usage when setting up CPI
parameters in a Pinocchio program that performs a CPI to a simple program that
logs the number of accounts received. This benchmark measures the improvement in
CPI parameter setup. All other costs of a CPI remain the same.

|  | CPI (8) | CPI (16) | CPI (32) | CPI (64) |
| --- | --- | --- | --- | --- |
| invoke | 1,376 | 1,592 | 1,999 | 2,813 |
| invoke v2 (proposed) | **1,174** | **1,198** | **1,221** | **1,267** |
| **Change invoke v2 vs invoke** | **−15%** | **−25%** | **−39%** | **−55%** |

## Security Considerations

The security surface is identical to that of the existing invoke syscall. This
proposal does not change the CPI logic in the runtime, apart from reading the
account information through the instruction account index during account
translation. The same input validation and CU metering apply.

## Backwards Compatibility

This proposal is fully backwards compatible. The new `invoke_signed_v2` syscall
is an addition to the existing `invoke_signed` syscall, which remains unchanged.
Programs that do not use the new syscall will continue to function as before.
