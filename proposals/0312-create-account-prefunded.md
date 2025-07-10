---
simd: '0312'
title: CreateAccountPrefunded
authors:
  - Peter Keay
category: Standard
type: Core
status: Review
created: 2025-06-27
feature: create-account-prefunded
---

## Summary

A `CreateAccountPrefunded` instruction added to the system program reduces
network overhead for applications that need to fund account rent (in whole
or in part) in advance of account creation.

## Motivation

The existing `CreateAccount` wrapper instruction creates accounts within a
program by use of `transfer`, `allocate`, and `assign`. However,
`CreateAccount` fails if `lamports > 0`. This was intended as a protection
measure, preventing developers from accidentally passing in some normal
wallet address and permanently locking its lamports after `allocate` and
`assign`.

However, it is common practice to provide rent lamports to accounts prior to
the actual creation (allocation and assigning) of the account space, rather
than forcing the fee payer of the transaction to provide all of the required
lamports. In this and similar instances, developers currently must manually
construct a patched `CreateAccount` call of their own with 2-3 CPI calls:
sometimes `Transfer`, then `Allocate`, and then `Assign`. While these
actions themselves are minimally expensive, the overhead incurred
with every Cross-Program Invocation - depth check, signer check,
account copy, etc. - can make up the bulk of the computation done in the
transaction. Each CPI incurs a minimum of 1_000 compute units, plus
additional amounts depending on the instruction and account length.

`CreateAccountPrefunded` performs `allocate`, `assign`, and `transfer`
without asserting that the created account has zero lamports. Applications
which do not need to `transfer` can specify 0 lamports, effectively providing
them with an `AllocateAndAssign`.

p-ATA program benchmarks demonstrate that use of `CreateAccountPrefunded`
saves approximately 2_500 compute units:
https://github.com/solana-program/associated-token-account/pull/102

This is a stopgap measure, as it extends the undesired interface sprawl of
helper instructions such as `CreateAccount`. However, any redesign of
Cross-Program Invocations to safely reduce overhead would be an extensive
protocol upgrade and slow to land. In the meantime, the network will benefit
from `CreateAccountPrefunded`.

## New Terminology

* Prefunded Account: an account which receives lamports to pay for its rent
in whole or in part before its space is `allocate`d and its owner is
`assign`ed.

## Detailed Design

`CreateAccountPrefunded` is added as a system program instruction, identical
to `CreateAccount` in all but discriminant (13).

```
/// # Account references
///   0. `[WRITE, SIGNER]` Funding account
///   1. `[WRITE, SIGNER]` New account
CreateAccountPrefunded {
    /// Number of lamports to transfer to the new account (can be 0)
    lamports: u64,

    /// Number of bytes of memory to allocate
    space: u64,

    /// Address of program that will own the new account
    owner: Pubkey,
},
```

Behavior is identical to `CreateAccount`, but does not fail if the
new account's `lamports > 0`.

## Alternatives Considered

* A separate `AllocateAndAssign` instruction. However, using
`CreateAccountPrefunded` is appropriate for a caller needing to `allocate`
and `assign`, as `transfer` is called to top up the storage rent
requirement only if current lamports are insufficient (equivalent to how an
instruction named `AllocateAndAssignAndMaybeTransfer` would function).
A separate `AllocateAndAssign` would save one check, but the compute savings
are likely not enough to justify the resulting interface sprawl.
* Redesigning CPIs. The current CPI model spins up a new context for every
invocation - re-copying and re-verifying account and signer data. A CPI
redesign ​could slash this overhead for innumerable programs, but such a
redesign would involve extensive implementation and audit time. By contrast,
`CreateAccountPrefunded` delivers quick compute savings for a common pattern
with minimal surface area. Instruction-batching helpers such as
`CreateAccountPrefunded` and `CreateAccount` can be deprecated whenever
CPI improvements land, enabling such helpers to become library-level functions
rather than system instructions.

## Drawbacks

Interface sprawl. As mentioned, this is a temporary compromise until
CPI improvements can land.

## Impact

The primary impact is an available reduction in CPI overhead for programs
which currently must perform these operations manually across 2 to 3
CPIs.

## Security Considerations

Developers using `CreateAccountPrefunded` should ensure they are not passing
an unintended wallet account with lamports. The usual checks of `allocate`
and `assign` are still performed - an account with data or with an owner
other than the system program will not be modified - but since the lamport
check is removed, a wallet account with lamports can be bricked by this
instruction.

## Backwards Compatibility

This SIMD requires a feature gate, as it adds a new System Program
instruction.
