---
simd: '0602'
title: Disallow Nonce Account as Program ID
authors:
  - Hanako Mumei (Anza)
category: Standard
type: Core
status: Review
created: 2026-08-18
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

A nonce transaction may not specify its nonce account as a program ID. Any
transaction that violates this condition is considered invalid and must be
discarded. A block that contains such a transaction is invalid.

## Motivation

Excluding nonces from being specified as program IDs is necessary to support
[SIMD-0192](https://github.com/solana-foundation/solana-improvement-documents/pull/192)
(ALT Relaxation). Our main goal is to unblock Async Execution / Bankless
Leaders.

In an attempt to decouple nonce validation and ALT resolution,
[SIMD-0242](https://github.com/solana-foundation/solana-improvement-documents/pull/242)
disallowed nonce accounts to come from ALTs. This did decouple the nonce
_account_ validation from ALT resolution. But the part that is supposed to only
depend on transaction bytes, without loading account data, _still_ depends on
ALT resolution, because the nonce must be marked writable on the transaction,
and ALT resolution can change the outcome of write-lock demotion by bringing
LoaderV3 into scope.

The purpose of this SIMD is to sever that relationship fully:

* Nonce transaction nonce accounts already cannot come from ALTs.
* Signers cannot come from ALTs, so we can verify authority without resolving.
* Program IDs cannot come from ALTs.
* Presently, a nonce account used as a program ID would be demoted to read-only,
producing an invalid nonce transaction. However, an ALT with LoaderV3 would
prevent that demotion, creating a consensus-relevant coupling between nonce
transaction validation and ALT resolution.

By banning nonces from being used as program IDs, we can determine whether a 
transaction is a semantically valid nonce transaction with just the transaction
bytes and the currently valid blockhashes, and we can fully validate the nonce
transaction with both plus the nonce account, without needing to resolve ALTs.

There is a notional separation between nonce validation that depends only on the
transaction bytes, versus nonce validation that depends on the stored on-chain
nonce account. This distinction is academic and meaningless until
[SIMD-0297](https://github.com/solana-foundation/solana-improvement-documents/pull/297)
(Nonce Relaxation) is amended and implemented, and presently all nonce
validation failures uniformly produce the same invalid/discard outcome.

## New Terminology

None, but for clarity:

ALT refers to Address Lookup Table.

A Nonce Transaction is defined as a transaction with a valid
`AdvanceNonceAccount` instruction and _does not_ have a valid blockhash. A
transaction with a valid blockhash is a Blockhash Transaction no matter what
instructions it carries.

A Blockhash Transaction by definition does not have a nonce account. Therefore,
the proposed rule does not apply to Blockhash Transactions.

The Nonce Account for a Nonce Transaction is the account designated by the
pubkey specified in the `AdvanceNonceAccount` instruction. In this SIMD, "nonce
account" refers to the account designated by a transaction under consideration,
not general properties of nonce accounts on chain.

The Next Durable Nonce is the durable nonce value computed off the current
bank's last blockhash.

A transaction's Lifetime Specifier is what legacy/V0 call the recent blockhash.
We borrow this term from the Transaction V1 spec because it is fully general and
avoids confusion.

A Current Initialized Nonce Account is a valid nonce account that begins with
the byte pattern `01 00 00 00 01 00 00 00`.

A Reserved Key is one of the consensus-enforced keys that are never allowed to
be writable, including but not limited to sysvars and builtin programs.

## Detailed Design

The existing nonce validation for consensus proceeds according to these rules,
_after_ determining the transaction does not have a valid blockhash:

1. The transaction lifetime specifier must not be the next durable nonce.
2. The transaction's first instruction must be a `AdvanceNonceAccount` call to
the System Program.
3. The nonce account pubkey must be a static account key and must be marked
writable on the transaction.
4. The nonce account must exist, be owned by System Program, parse successfully
as a Current Initialized nonce account, and the stored nonce value must match
the transaction lifetime specifier.
5. The nonce account authority must be a signer on the `AdvanceNonceAccount`
instruction.

The order of the above steps is non-strict: the failure of any condition results
in the same outcome, an invalid nonce transaction that must not be committed to
chain or else violate the nonce block constraint.

We can very easily check that nonce account is not a program ID during point 3.
The new condition would likewise result in an invalid nonce transaction. The
restriction cannot be part of sanitization because whether a transaction is a
"nonce transaction" depends on the recent blockhashes at the time it is
processed.

This allows us to fully determine whether the nonce account is writable or read-
only without resolving ALTs as a consequence of the fact that it would never be
subject to program ID demotion. As an implementation note, we should still check
that the nonce is not a reserved key or perform demotion on the static
transaction; we _must_ do this if we decouple static validation from nonce-
account-dependent validation. Otherwise it is an implementation detail because
no reserved key account may ever be a valid nonce account.

## Alternatives Considered

We could in theory accomplish
[SIMD-0192](https://github.com/solana-foundation/solana-improvement-documents/pull/192)
without adding the program ID special case to nonces. This would however be far
more brittle, because we would need to mix nonce validation and ALT validation,
because nonce validity would depend on ALT contents, and landing an invalid ALT
transaction as fee-only would depend on nonce validity. Decoupling these is
highly desirable.

We could unconditionally demote the nonce account to read-only regardless of the
contents of the ALT. This however would be an exception to an exception and
create new nonce-ALT conceptual overlap. Banning nonce transactions from
declaring their nonce account as a program ID depends only on transaction bytes
and mirrors the existing fee-payer program ID restriction.

We could do nothing and wait for nonce removal. Frankly, we are moving forward
with ALT Relaxation so we can make progress on Bankless Leaders while killing
time to see if nonce removal might solve major issues blocking Nonce Relaxation.
We should not wait for nonce removal here too.

We could ban adding LoaderV3 in `ExtendLookupTable` and regard any ALT
containing it as invalid. Unlike the previous three options, this is perfectly
reasonable and a viable alternative SIMD to this one, if so desired by others. 
This would brick 213 out of 2,205,707 ALTs that currently exist on Mainnet-Beta
as of Epoch 1018, or 0.0097% of ALTs. It would however, unlike this SIMD,
potentially affect real users.

We could also remove program ID write-lock demotion from the protocol entirely.
This would moot the issue by decoupling nonce writable status from ALT
resolution. We may, but do not need to, remove reserved key demotion as well.
The reserved key case is trivial to handle with regards to nonce accounts
without involving ALTs.

## Impact

Realistically, none. A transaction that specifies its nonce as a program ID
without this SIMD will either be discarded, or land as fee-only, depending on
write-lock demotion. This proposal merely ensures such a transaction is
uniformly discarded.

## Security Considerations

None.

## Backwards Compatibility

Transactions which may previously have been fee-only become invalid.

## Conformance

Validator clients should test that a nonce transaction which includes LoaderV3
via an ALT, and uses its nonce account as a program ID, is dropped rather than
processed as fee-only.
