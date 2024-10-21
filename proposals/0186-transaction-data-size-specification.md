---
simd: '0186'
title: Transaction Data Size Specification
authors:
  - Hanako Mumei
category: Standard
type: Core
status: Review
created: 2024-10-20
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Before a transaction can be executed, every account it may read from or write to
must be loaded, including any programs it may call. The amount of data a
transaction is allowed to load is capped, and if it exceeds that limit, loading
is aborted. This functionality is already implemented in the validator. The
purpose of this SIMD is to explicitly define how loaded transaction data size is
calculated.

## Motivation

Transaction data size accounting is currently unspecified, and the
implementation-defined algorithm used in the Agave client exhibits some
surprising behaviors:

* BPF loaders required by top-level programs are counted against transaction
data size. BPF loaders required by CPI programs are not. If a required BPF
loader is also included in the accounts list, it is counted twice.
* The size of a program owned by LoaderV3 may or may not include the size of its
programdata depending on how the program account is used on the transaction.
Programdata is also itself counted if included in the transaction accounts list.
This means programdata may be counted zero, one, or two times per transaction.

All validator clients must arrive at precisely the same transaction data size
for all transactions because a difference of one byte can determine whether a
transaction is executed or failed, and thus affects consensus. Also, we want the
calculated transaction data size to correspond well with the actual amount of
data the transaction requests.

Therefore, this SIMD seeks to specify an algorithm that is straightforward to
implement in a client-agnostic way, while also accurately accounting for the
total data required by the transaction.

## New Terminology

One term is defined within the scope of this SIMD:

* Valid program: a program that has been loaded, or a builtin. This definition
excludes programs that have failed verification, or LoaderV3 programs that have
been closed or have delayed visibility due to being deployed or modified in the
current slot.

These terms are not new, however we define them for clarity:

* Top-level program: the program corresponding to the program id on a given
instruction.
* Instruction account: an account passed to an instruction, which allows its
program to view the actual bytes of the account. CPI can only happen through
programs provided as instruction accounts.
* Transaction accounts list: all accounts for the transaction, which includes
top-level programs, the fee-payer, all instruction accounts, and any extra
accounts added to the list but not used for any purpose.

## Detailed Design

The proposed algorithm is as follows:

1. Every account explicitly included on the transaction accounts list is counted
once and only once.
2. A valid program owned by LoaderV3 also includes the size of its programdata.
3. Other than point 2, no accounts are implicitly added to the total data size.

Transactions may include a
`ComputeBudgetInstruction::SetLoadedAccountsDataSizeLimit` instruction to define
a data size limit for the transaction. Otherwise, the default limit is 64MiB
(`64 * 1024 * 1024` bytes).

If a transaction exceeds its data size limit, account loading is aborted and the
transaction is failed. Fees will be charged once
`enable_transaction_loading_failure_fees` is enabled.

Adding required loaders to transaction data size is abolished. They are treated
the same as any other account: counted if on the transaction accounts list, not
counted otherwise.

Read-only and writable accounts are treated the same. In the future, when direct
mapping is enabled, this SIMD may be amended to count them differently.

As a consequence of 1 and 2, for LoaderV3 programs, programdata is counted twice
if a transaction includes both programdata and the program account itself in the
accounts list, unless the program is not valid for execution. This is partly
done for ease of implementation: we always want to count programdata when the
program is included, and there is no reason for any transaction to include both
accounts except during initial deployment, in which case the program is not yet
valid.

We include programdata size in program size for LoaderV3 programs because in
nearly all cases a transaction will include the program account (the only way to
invoke the program) and will not include the programdata account because
including it serves no purpose. Including the program account forces an
unconditional load of the programdata account because it is required to compile
the program for execution. Therefore we always count it, even when the program
is an instruction account.

There is no special handling for programs owned by the native loader, LoaderV1,
or LoaderV2.

Account size for programs owned by LoaderV4 is left undefined. This SIMD should
be amended to define the required semantics before LoaderV4 is enabled on any
network.

## Alternatives Considered

* Transaction data size accounting is already enabled, so the null option is to
enshrine the current Agave behavior in the protocol. This is undesirable because
the current behavior is highly idiosyncratic, and LoaderV3 program sizes are
routinely undercounted.
* Builtin programs are backed by accounts that only contain the program name as
a string, typically making them 15-40 bytes. We could make them free when not
instruction accounts, since they're part of the validator. However this
adds complexity for no real benefit.

## Impact

The primary impact is this SIMD makes correctly implementing transaction data
size accounting much easier for other validator clients.

It makes transactions which include program accounts for CPI somewhat larger,
but given the generous 64MiB limit, it is unlikely that any existing users will
be affected.

## Security Considerations

Security impact is minimal because this SIMD merely simplifies an existing
feature.

This SIMD requires a feature gate.

## Backwards Compatibility

Transactions that currently have a total transaction data size close to the
64MiB limit, which call LoaderV3 programs via CPI, may now exceed it and fail.
