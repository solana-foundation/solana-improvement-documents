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

A `CreateAccountPrefunded` instruction reduces network overhead for
applications that need to fund account rent (in whole or in part) in advance
of account creation.

## Motivation

The existing `CreateAccount` wrapper instruction creates accounts within a
program by use of `transfer`, `allocate`, and `assign`. However,
`CreateAccount` fails if `lamports == 0`. This was intended as a protection
measure, preventing developers from accidentally passing in some normal
wallet address and permanently locking its lamports after `allocate` and
`assign`.

However, it is common practice to provide rent lamports to accounts prior to
the actual creation (allocation and assigning) of the account space, rather
than forcing the `payer` of the ATA account creation transaction to provide
all of the required lamports. In this and similar instances, developers must
manually construct a patched `CreateAccount` call of their own with 2-3 CPI
calls: sometimes `Transfer`, then `Allocate`, and then `Assign`. While these
actions themselves are minimally expensive, the overhead incurred with every
Cross-Program Invocation - depth check, signer check, account copy, etc. -
can make up the bulk of the computation done in the transaction.

`CreateAccountPrefunded` performs `allocate`, `assign`, and `transfer`
without asserting that the created account has zero lamports. Applications
which do not need to `transfer` can specify 0 lamports, effectively providing
them with an `AllocateAndAssign`.

This is a stopgap measure, as it extends the undesired interface sprawl of
helper instructions such as `CreateAccount`. However, any redesign of
Cross-Program Invocations to safely reduce overhead would be an extensive
protocol upgrade and slow to land. In the meantime, the network will benefit
from `CreateAccountPrefunded`.

## New Terminology

No new terms are introduced by this SIMD, however we define these for clarity:

* Prefunded: an account which receives lamports to pay for its rent in whole
or in part before its space is `allocate`d and its owner is `assign`ed.

## Detailed Design

`create_account_prefunded()` takes the same arguments as `create_account()`
and performs the same operations; however, it does not fail if the new
account's `lamports > 0`.

## Alternatives Considered

* A separate `AllocateAndAssign` instruction. Multiple-instruction helpers
like this incur short-term interface sprawl, at least ahead of future CPI
improvements, but these are not just helpers for developer ergonomics; they
prevent common activities from causing repetitive computation. Using
`CreateAccountPrefunded` and specifying 0 lamports as the amount to transfer
reduces interface sprawl without incurring any appreciable compute cost.

## Drawbacks

Interface sprawl

## Impact

The primary impact is a reduction in CPI overhead for applications which
currently perform these operations manually across 2 to 3 instructions. Of
the top 10 programs on Solana as of this writing, all 10 take this approach.

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
