---
simd: '0312'
title: CreatePrefundedAccount
authors:
  - Peter Keay
category: Standard
type: Core
status: Review
created: 2025-06-27
feature: create-account-prefunded
---

## Summary

A `CreatePrefundedAccount` instruction added to the system program reduces
network overhead for applications that need to fund account rent (in whole
or in part) in advance of account creation.

## Motivation

The existing `CreateAccount` system program instruction creates accounts
within a program by use of `transfer`, `allocate`, and `assign`. However,
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

`CreatePrefundedAccount` performs `allocate`, `assign`, and `transfer`
without asserting that the created account has zero lamports. Applications
which do not need to `transfer` can specify 0 lamports, effectively providing
them with an `AllocateAndAssign`.

p-ATA program benchmarks demonstrate that use of `CreatePrefundedAccount`
can save approximately 2_500 compute units:
https://github.com/solana-program/associated-token-account/pull/102

This is a stopgap measure, as it extends the undesired interface sprawl of
helper instructions such as `CreateAccount`. However, any redesign of
Cross-Program Invocations to safely reduce overhead would be an extensive
protocol upgrade and slow to land. In the meantime, the network will benefit
from `CreatePrefundedAccount`.

## New Terminology

* Prefunded Account: an account which has received lamports to pay for its rent
in whole or in part in any prior instruction or prior transaction, before the
instruction in which its space is `allocate`d and its owner is `assign`ed.

## Detailed Design

`CreatePrefundedAccount` is added as a system program instruction, identical
to `CreateAccount` except for the following:

1. The instruction has a new discriminant (13).
2. The funding account is optional.
3. Due to #2, the accounts are ordered differently.

### Instruction Data

| Field | Type | Description |
|---|---|---|
| Discriminant | `u32` | The instruction discriminant, value `13`. |
| `lamports` | `u64` | Number of lamports to transfer to the new account. |
| `space` | `u64` | Number of bytes of memory to allocate. |
| `owner` | `Pubkey` | Address of program that will own the new account. |

### Accounts

The instruction requires the following accounts:

| Index | Role | Description |
|---|---|---|
| 0 | `[WRITE, SIGNER]` | **New account**: The account to be created. |
| 1 | `[WRITE, SIGNER] (optional)` | **Funding account**: The account that
will pay for the lamport transfer. Required only when `lamports > 0`. |

### Behavior

The `CreatePrefundedAccount` instruction performs the following actions:

1. **Allocate**: As with `CreateAccount`, this instruction calls `allocate`,
which will fail if the new account is non-empty.

2. **Assign**: As with `CreateAccount`, this instruction calls `assign`.

3. **Transfer**: If `lamports` is greater than 0, it transfers the
specified number of lamports from the `funding_account` (account index 1)
to the `new_account` (account index 0).
`lamports` can be used when the account is prefunded insufficiently; in other
words, when the account has some lamports, but needs more to cover rent.

This instruction will fail if:

* The `funding_account` is not provided when `lamports > 0`.

Additional underlying reasons may cause the instruction to fail; in other
words, it will fail if `allocate`, `assign`, or `transfer` fail for any
reason including:

* The `funding_account` does not have enough lamports for the transfer (when
`lamports > 0`).
* The `new_account` already contains data or is not owned by the System
Program.
* The `new_account` does not have sufficient lamports to be rent-exempt
after the transfer.
* Required accounts are not writable or not signers.
* The requested `space` exceeds the max permitted data length. 

## Alternatives Considered

* A separate `AllocateAndAssign` instruction. However, using
`CreatePrefundedAccount` is appropriate for a caller needing to `allocate`
and `assign`, as `transfer` is called to top up the storage rent
requirement only if current lamports are insufficient (equivalent to how an
instruction named `AllocateAndAssignAndMaybeTransfer` would function).
A separate `AllocateAndAssign` would save one check, but the compute savings
are not enough to justify the resulting interface sprawl.
* Redesigning CPIs. The current CPI model spins up a new context for every
invocation - re-copying and re-verifying account and signer data. A CPI
redesign ​could slash this overhead for innumerable programs, but such a
redesign would involve extensive implementation and audit time. By contrast,
`CreatePrefundedAccount` delivers quick compute savings for a common pattern
with minimal surface area. Instruction-batching helpers such as
`CreatePrefundedAccount` and `CreateAccount` can be deprecated whenever
CPI improvements land, enabling such helpers to become library-level functions
rather than system instructions.

## Drawbacks

Interface sprawl. As mentioned, this is a temporary compromise until
CPI improvements can land.

## Impact

The primary impact is an available reduction in CPI overhead for programs
which currently must perform these operations manually across 2 to 3
CPIs.

As mentioned previously, the p-ATA program takes advantage of
`CreatePrefundedAccount` to save approximately 2_500 compute units:
https://github.com/solana-program/associated-token-account/pull/102

## Security Considerations

Developers using `CreatePrefundedAccount` should ensure they are not passing
an unintended wallet account with lamports. The usual checks of `allocate`
and `assign` are still performed - an account with data or with an owner
other than the system program will not be modified - but since the lamport
check is removed, a wallet account with lamports can be bricked by this
instruction.

## Backwards Compatibility

This SIMD requires a feature gate, as it adds a new System Program
instruction.
