---
simd: '0186'
title: Loaded Transaction Data Size Specification
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
is aborted. This functionality is already implemented in the validator.

This SIMD defines a new algorithm for calculating the consensus-enforced total
size of loaded transaction data during transaction processing.

## Motivation

Transaction data size accounting is currently unspecified, and the
implementation-defined algorithm used in the Agave client exhibits some
surprising behaviors:

* BPF loaders required by instructions' program IDs are counted against
transaction data size. BPF loaders required by CPI programs are not. If a
required BPF loader is also included in the accounts list, it is counted twice.
* The size of a program owned by LoaderV3 may or may not include the size of its
programdata depending on how the program account is used on the transaction.
Programdata is also itself counted if included in the transaction accounts list.
This means programdata may be counted zero, one, or two times per transaction.
* Due to certain quirks of implementation, loader-owned accounts which do not
contain valid programs for execution may or may not be counted against the
transaction data size total depending on how they are used on the transaction.
This includes, but is not limited to, LoaderV3 buffer accounts, and accounts
which fail ELF validation.
* Accounts can be included on a transaction account list without being an
instruction account, fee-payer, or program ID. These accounts are presently
loaded and counted against transaction data size, although they can never be
used for any purpose by the transaction.

All validator clients must arrive at precisely the same transaction data size
for all transactions because a difference of one byte can determine whether a
transaction is executed or failed, and thus affects consensus. Also, we want the
calculated transaction data size to correspond well with the actual amount of
data the transaction requests.

Therefore, this SIMD seeks to specify an algorithm that is straightforward to
implement in a client-agnostic way, while also accurately accounting for all
account data required by the transaction.

## New Terminology

No new terms are introduced by this SIMD, however we define these for clarity:

* Instruction account: an account passed to an instruction in its accounts
array, which allows the program to view the actual bytes contained in the
account. CPI can only happen through programs provided as instruction accounts.
* Transaction accounts list: all accounts for the transaction, which includes
instruction accounts, the fee-payer, program IDs, and any extra accounts added
to the list but not used for any purpose.
* LoaderV3 program account: an account owned by
`BPFLoaderUpgradeab1e11111111111111111111111` which contains in its account data
the first four bytes `02 00 00 00` followed by a pubkey which points to an
account which is defined as the program's programdata account.

For the purposes of this SIMD, we make no assumptions about the contents of the
programdata account.

## Detailed Design

The proposed algorithm is as follows:

1. Given a transaction, take the unique set of account keys which are used as:
    * An instruction account.
    * A program ID for an instruction.
    * The fee-payer.
2. Each account's size is determined solely by the byte length of its data prior
to transaction execution, irrespective of it is used on the transaction.
3. For any loaded account identified as a `LoaderV3` program account, add the
size of the programdata account it references to its own size, irrespective of
how the program account is used on the transaction.
4. The total transaction size is the sum of these sizes.

Transactions may include a
`ComputeBudgetInstruction::SetLoadedAccountsDataSizeLimit` instruction to define
a lower data size limit for the transaction. Otherwise, the default limit is
64MiB (`64 * 1024 * 1024` bytes).

If a transaction exceeds its data size limit, the transaction is failed. Fees
will be charged once `enable_transaction_loading_failure_fees` is enabled.

Adding required loaders to transaction data size is abolished. They are treated
the same as any other account: counted if used in a manner described by 1, not
counted otherwise.

No account that falls outside of the three categories listed by 1 is counted
against transaction data size. Validator clients are free to decline to load
them.

Read-only and writable accounts are treated the same. In the future, when direct
mapping is enabled, this SIMD may be amended to count them differently.

As a consequence of 1 and 3, for LoaderV3 programs, programdata is counted twice
if a transaction explicitly references the program account and its programdata
account. This is done partly for simplicity, and partly to account for the cost
of maintaining the compiled program in addition to the actual bytes of
the programdata account.

We include programdata size in account size for LoaderV3 programs because using
the program account on a transaction forces an unconditional load of programdata
to compile the program for execution. We always count it, even when the program
is an instruction account, because the program must be available for CPI.

There is no special handling for any account owned by the native loader,
LoaderV1, or LoaderV2.

Account size for programs owned by LoaderV4 is left undefined. This SIMD should
be amended to define the required semantics before LoaderV4 is enabled on any
network.

## Alternatives Considered

* Transaction data size accounting is already enabled, so the null option is to
enshrine the current Agave behavior in the protocol. This is undesirable because
the current behavior is highly idiosyncratic, and LoaderV3 program sizes are
routinely undercounted.
* Builtin programs are backed by accounts that only contain the program name as
a string, typically making them 15-40 bytes. We could impose a larger fixed cost
for these. However, they must be made available for all programs anyway, and
most of them are likely to be ported to BPF eventually, so this adds complexity
for no real benefit.
* Several slightly different algorithms were considered for handling LoaderV3
programs in particular, for instance only counting programs that are valid for
execution in the current slot. However, this would implicitly couple transaction
data size with the results of ELF validation, which is highly undesirable.
* We considered loading and counting sizes for accounts on the transaction
account list which are not used for any purpose. This is the current behavior,
but there is no reason to load such accounts at all.

## Impact

The primary impact is this SIMD makes correctly implementing transaction data
size accounting much easier for other validator clients.

It makes the calculated size of transactions which include program accounts for
CPI somewhat larger, but given the generous 64MiB limit, it is unlikely that any
existing users will be affected. Based on an investigation of a 30-day window,
transactions larger than 30MiB are virtually never seen.

## Security Considerations

Security impact is minimal because this SIMD merely simplifies an existing
feature. Care must be taken to implement the rules exactly.

This SIMD requires a feature gate.

## Backwards Compatibility

Transactions that currently have a total transaction data size close to the
64MiB limit, which call LoaderV3 programs via CPI, may now exceed it and fail.
