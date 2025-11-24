---
simd: 'XXXX'
title: Nonce Payload Transaction Format
authors:
  - Hanako Mumei
category: Standard
type: Core
status: Idea
created: 2025-11-23
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

A new transaction format which wraps legacy nonce transactions with an external
fee-payer and blockhash, allowing them to be processed as normal blockhash
transactions.

## Motivation

Nonce transactions have been a perennial problem for validator clients:

1. Nonce transactions have no inherent TTL, and thus are valid until executed.
2. Nonce transactions are impossible to validate without knowing the nonce
account state immediately prior to execution, complicating edge filtration.
3. Nonce transactions which fail to validate must be discarded without paying
fees, to prevent transaction replay.

The combination of these three properties makes them an attractive vehicle for
transaction spam, whether malicious (to degrade network performance), or merely
rude (to increase chances of inclusion during network congestion at no cost to
the sender).

In addition,
[SIMD-0297](https://github.com/solana-foundation/solana-improvement-documents/pull/297)
proposes to allow transactions which fail nonce validation to be committed to
chain as no-ops. This has the unfortunate side-effect of making committed
transaction signatures non-unique because it is impossible to determine whether
a nonce transaction has been committed previously if the nonce account is not
mutated. By eliminating top-level nonce transactions, we can preserve signature
uniqueness.

We propose a new transaction format that is effectively an additional header
preceding a full legacy nonce transaction. Users will provide a fee-payer,
recent blockhash, compute budget options, legacy nonce transaction, and a
signature over this complete message. Compared to alternative designs, this has
a number of attractive properties:

1. All existing legacy nonce transactions remain processable as blockhash
transactions through this new mechanism. This allows us to rapidly deprecate and
remove nearly all special handling of nonce transactions from validator clients
while providing a path for full backwards compatibility.
2. No authorities have to be reassigned. No offline flows need to change. Users
produce a legacy nonce transaction the same way they always did. The only
workflow change is to drop it into a new CLI or wallet flow which wraps the
payload and signs with a hot wallet.
3. No new signer promotion rules are required in the runtime.
4. The ultimate fee-payer does not need to be known when the nonce payload is
produced and can be chosen arbitrarily prior to submission for execution.

As a side benefit, nonce payload transactions become the obvious best choice for
sponsored transactions. Users do not need to designate an explicit fee-payer so
the proxy can choose one arbitrarily, and attacks using the third-party
fee-payer as a signer become impossible because the inner transaction has no
access to the outer authority.

## Dependencies

This proposal depends on the following (not yet previously accepted) proposals:

- **[SIMD-0296]: Larger Transaction Size**

    A legacy transaction may be up to 1232 bytes. We take advantage of larger
    transaction sizes to wrap legacy transactions.

[SIMD-0296]: https://github.com/solana-foundation/solana-improvement-documents/pull/296

## New Terminology

* Nonce Payload Transaction, the name of this transaction format as a whole.
* Nonce Envelope, a shorthand for the header which precedes the legacy
transaction.
* Nonce Payload, a shorthand for the legacy transaction in isolation.

## Detailed Design

### Nonce Payload Transaction Specification

The binary layout of a Nonce Payload Transaction is as follows:

XXX repeat version byte inside?

```
VersionByte (u8)
FeePayerAddress [u8; 32]
RecentBlockhash [u8; 32]
TransactionConfigMask (u32) -- Bitmask of which config requests are present.
LegacyTransactionLength (u16)
LegacyNonceTransaction [u8] -- Length equal to the serialized nonce payload, at
  most 1232 bytes.
ConfigValues [[u8; 4]] -- Length equal to the popcount (number of set bits)
  of TransactionConfigMask. See section TransactionConfigMask for details.
Signature [u8; 64] -- External signature over the entire message
```

#### VersionByte

The VersionByte MUST be set to `130` to distinguish Nonce Payload Transactions
from legacy/v0/v1 formats.

#### FeePayerAddress

The pubkey of the account which will pay fees on behalf of this transaction.
Note that the `LegacyNonceTransaction` itself _does not_ have a fee-payer per
se; its first signer has no special significance.

#### RecentBlockhash

A recent blockhash, identical in meaning to legacy and v0 blockhash
transactions.

#### TransactionConfigMask

`TransactionConfigMask` is explained below in detail.

#### LegacyTransactionLength

2-byte LE u16 encoding the length in bytes of the enveloped legacy nonce
transaction.

#### LegacyNonceTransaction

A legacy nonce transaction that adheres to all sanitization rules of a normal
legacy transaction. This transaction MUST have `AdvanceNonceAccount` as its
first instruction, otherwise it is a sanitization failure. It MUST NOT contain
trailing bytes, otherwise it is a sanitization failure.

#### ConfigValues

`ConfigValues` is explained below with `TransactionConfigMask` in detail. It is
placed after `LegacyNonceTransaction` to allow the nonce payload to sit at a
fixed offset for the benefit of SigVerify.

#### Signature

One signature, by the private key associated with `FeePayerAddress`, over the
full serialized message including `LegacyNonceTransaction`.

### TransactionConfigMask

The transaction config mask is used to configure specific fee and resource
requests in a transaction. It is identical to the field as proposed in
[SIMD-0385](https://github.com/solana-foundation/solana-improvement-documents/pull/385)
and intended to allow both transaction formats to be enriched with new features
as desired. There is however no requirement that new config options in one
format be mirrored in the other.

Initially supported fields and assigned bits:

- [0, 1] - total lamports for transaction priority fee. 8-byte LE u64.
- [2] - compute unit limit. 4-byte LE u32.
- [3] - requested loaded accounts data size limit. 4-byte LE u32.
- [4] - requested heap size. 4-byte LE u32.

For 2 bit fields, such as priority-fee, both bits MUST be set. If only one of 
the bits is set, the transaction is invalid and cannot be included in blocks.

All `ComputeBudgetProgram` instructions on the nonce payload are ignored, even
if they are invalid. This design is intended to allow the external fee-payer to
determine what priority fee and resource limits they are willing to pay.

### SigVerify

The nested nature of a Nonce Payload Transaction requires special handling by
SigVerify. `Signature` is to be verified over the entire signed message using
the pubkey specified in `FeePayerAddress`. Signatures inside
`LegacyNonceTransaction` must be verified against that message itself.
`FeePayerAddress` and `LegacyNonceTransaction` start at fixed offsets, which
minimizes the computational cost to the greatest extent possible.

### Transaction Processing

As noted above, the inner transaction MUST be a valid nonce transaction.
Allowing a legacy blockhash transaction to serve as a payload is fundamentally
unsound, as there is no mechanism to prevent replay.

When a Nonce Payload Transaction is sanitized, `FeePayerAddress` is used as the
fee-payer, `Signature` is used as the signature, `RecentBlockhash` is used as
the blockhash, `TransactionConfigMask` and `ConfigValues` are used to determine
priority fee and resource limits. The message hash is the hash of the bytes
signed by the external fee-payer. All instructions, accounts, and
writable/signer designations are as in `LegacyNonceTransaction`. This payload is
executed normally as if it were the full transaction itself. The payload's own
`RecentBlockhash` is used as the nonce value just as a legacy nonce transaction.
The payload cannot see its external fee-payer and has no access to this account
during execution, unless it provided the account explicitly, in which case the
account is not a signer unless it also properly signed the legacy transaction.

If account loading or transaction execution of the nonce payload fails for any
reason _other_ than inability to properly advance the nonce, it is processed as
a normal fee-only transaction: the external fee-payer pays fees, and the nonce
account is advanced to the next durable nonce. This prevents speculative replay
of a nonce payload, where it is repeatedly run until it succeeds. This mechanism
to advance the nonce account in light of execution failure is the _only_ runtime
support for nonce transactions required.

If account loading or transaction execution of the nonce payload fails, and it
is _not_ possible to advance the nonce account for any reason (not a valid nonce
account, authority is not a signer, nonce account value does not match
transaction nonce value, already advanced this slot), the external fee-payer
pays fees and no other action is taken. This acts as a disincentive for
submitting spurious Nonce Payload Transactions, and is safe from the perspective
of the payload author.

When
[SIMD-0290](https://github.com/solana-foundation/solana-improvement-documents/pull/290)
is enabled, if the external fee-payer is invalid, the transaction MAY be
committed to chain. Such a transaction induces no state changes, including to
the payload nonce account. This means a future transaction could successfully
execute the payload, but because Nonce Payload Transactions are normal blockhash
transactions, such a transaction will necessarily have a unique signature.

This proposal neither depends on, nor blocks or supersedes,
[SIMD-0297](https://github.com/solana-foundation/solana-improvement-documents/pull/297),
which deals exclusively with top-level nonce transactions. SIMD-0297 may be
obviated if we fully remove nonce transactions after implementing this proposal,
but such an action is out of scope here and would depend on a future SIMD.

## Alternatives Considered

As always, the first alternative is to do nothing. This is highly undesirable
because nonce transactions are a constant thorn with impacts throughout the
transaction processing pipeline.

The leading alternative proposal for replacing nonce transactions is PDA signer
promotion, which would allow an instruction to promote a PDA to function as a
signer from the point of view of all subsequent instructions on the transaction.
However, this has several drawbacks:

1. Existing nonce transactions must be supported for an unknown length of time,
potentially years. When we do remove support for nonce transactions, it is
possible users who depended on their perpetual support may be unable to access
funds or perform some other essential operation.
2. Users of nonce transactions would need to reassign all keypair-based cold
authorities to associated PDAs. This may be a hard sell for conservative
organizations due to inertia, operational risk, and the increased risk surface
of granting an onchain program control over mints, program upgrades, etc.
3. Cross-instruction signer promotion has unknown consequences for existing
programs, which may have been designed under the assumption a PDA signer is only
possible via CPI.
4. PDA signer promotion requires a final hot signer and a hot wallet to pay
fees, so there is no advantage over this proposal in allowing fully cold
transaction creation. However, PDA signer promotion would also require teams to
upgrade their cold toolchains to support the new instruction, whereas this
proposal would not.

A stricter nonce program and stricter rules around valid nonce instruction
placement in transactions would make it feasible to determine if nonce
transactions are valid at time of batching. It would not, however, solve most of
the difficulties such transactions cause, which are inherent in the fact that
they do not carry a real blockhash enforcing a TTL.

We may want to consider supporting v0 transactions for the nonce payload, in
addition to legacy transactions. This was not done for simplicity, and because
we hope to move away from v0 transactions entirely due to address lookup tables.
This would mean that existing v0 nonce transactions become impossible to execute
once nonce support is removed from validator clients. However, there does not
seem to be a legitimate usecase for such transactions, as the typical user
relies on such transactions for simple operations that can easily fit into the
legacy transaction account limit.

## Impact

This proposal allows us to begin sunsetting nonce transactions. When that
process begins, ecosystem teams using legacy nonce transactions should be
prepared to sign for them with a hot wallet prior to submitting them to the
chain for execution.

Wallets should provide a flow for dropping a serialized legacy nonce transaction
into a field which wraps them in a Nonce Payload Transaction to be signed and
submitted.

## Security Considerations

Because the external fee-payer controls the compute budget parameters, it is
possible for them to "burn" a payload by setting artificially low limits,
forcing an execution failure that advances the nonce. This is unlikely to be a
concern in practice: in most cases, the same user produces the payload and the
final transaction, and the distinction is just to allow separate cold and hot
signing steps. If provided to a proxy, it is expected the nonce payload contains
a tip instruction to compensate for fees paid, so it would be economically
irrational to pay to induce a deliberate failure.

It is possible for a Nonce Payload Transaction to land onchain and pay fees but
fail to advance the nonce. If this is because the nonce account was invalid,
already used, or did not match the payload-defined nonce value, such a payload
can never become valid. However, if the nonce failed to advance because the
provided authority was incorrect, and the account's authority was later changed
to the authority given on the payload, such a payload could become executable.
This seems unlikely to occur as it would require deliberate user manipulation.
However, it could also be obviated by changing `AuthorizeNonceAccount` to
idempotently update the account to the latest durable nonce.

## Drawbacks

The need to verify two levels of signature over two different buffers
complicates SigVerify somewhat. However, this should be more than adequately
compensated by the ability to filter all transactions by blockhash validity
after legacy nonce transactions are removed.

In contrast to legacy nonce transactions, Nonce Payload Transactions require a
hot signer able to pay fees. This is inherent in any nonce-like blockhash-based
solution.
