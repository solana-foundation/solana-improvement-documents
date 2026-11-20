---
simd: '0723'
title: Remove Durable Nonces
authors:
  - Gabe R. (Anza)
category: Standard
type: Core
status: Draft
created: 2026-11-05
feature: (to be assigned upon acceptance)
---

## Summary

This proposal removes [durable nonces][original-nonce-proposal], which allow a
transaction to remain valid without a recent blockhash. The special cases they
require, from transaction validation through replay, have repeatedly caused
security vulnerabilities and consensus bugs in validator clients.

After activation, every transaction needs a recent blockhash. For offline and
delayed signing use cases, programs that verify signatures onchain can take the
place of durable nonces. Nonce account instructions and fee-payer eligibility
are unchanged.

## Motivation

A transaction expires once its recent blockhash is more than 151 blocks old.
Durable nonces were [proposed][solana#6725] and [implemented][solana#6845] in
2019 to support offline and delayed signing. They require additional consensus
rules:

1. A nonce transaction is valid only while its nonce account stores the same
   value as the transaction's blockhash field.
2. A failed nonce transaction must keep exactly two changes, the fee debit and
   the advanced nonce.

Every client must preserve these rules across transaction validation,
scheduling, execution, commit, caching and replay. Changes in one layer can
break assumptions in another. The following documented incidents and findings
illustrate the maintenance burden of these rules:

- **2019, vulnerability.** Failed durable nonce transactions could be replayed
  to drain fees ([solana#7443]).
- **2020, vulnerability.** The failure path persisted arbitrary account writes
  ([Neodyme]).
- **2022, mainnet incident.** A failed transaction was processed through both
  the blockhash and nonce paths, halting mainnet for about 4.5 hours ([Solana
  report]).
- **2024 to 2026, Firedancer bugs.** Fixes addressed nonce derivation, failure
  handling, bundle state and fee-only accounting ([fd#1500], [fd#9442],
  [fd#9512]).
- **2025, failure-handling bug.** Precomputed nonce rollback state was stale for
  conflicting batches ([agave#9455]).
- **2026, testnet incident.** A leader-only nonce filter was applied during
  replay and rejected a block that other clients accepted ([Jito release]).
- **2026, status-cache bug.** Nonce lifetimes produced unbounded cache buckets
  and a prospective cross-client hazard ([agave#14123]).

The exception also shapes other protocol work. [SIMD-0242] restricted nonce
accounts to static keys so account resolution could be relaxed. [SIMD-0602]
proposes also barring them as program IDs for the same reason. [SIMD-0290] was
narrowed to exclude durable nonce transactions. [SIMD-0297] exists because
state-dependent nonce failures currently invalidate blocks, which hinders
asynchronous execution.

Durable nonces also invite spam. A sender can sign many distinct transactions
against one nonce value. Once one lands, the rest pay nothing, yet validators
still receive and check them ([validator report][agave#8970]).

The goal is to remove the lifetime exception instead of maintaining the special
cases it requires. After activation, every transaction follows the same lifetime
and failure rules.

## New Terminology

- **Durable nonce transaction (DNT)**: a transaction whose first instruction is
  the System Program's `AdvanceNonceAccount` and whose recent blockhash is not
  in the bank's blockhash queue within the maximum processing age. Before
  activation, only its nonce account can make it valid.

## Detailed Design

### Overview

| Area | Before activation | After activation |
| --- | --- | --- |
| Lifetime | Recent blockhash or durable nonce | Recent blockhash only |
| Signed DNT | Valid while its nonce matches | `BlockhashNotFound` |
| Nonce instructions | Available | Unchanged |
| Nonce fee payers | Eligible | Unchanged |

### Transaction lifetime

A transaction's lifetime is valid while its recent blockhash is in the bank's
blockhash queue with an age of at most the maximum processing age, currently
151. In v1 messages this field is `LifetimeSpecifier`. Validators MUST NOT use a
nonce account, the contents of instruction 0 or a nonce authority signature to
decide lifetime validity.

A transaction without a valid lifetime is rejected in the same way as a
transaction with an expired blockhash. Its lifetime check fails with
`TransactionError::BlockhashNotFound` and no fee is charged. A block that
includes it is invalid.

Every transaction with a valid lifetime is therefore a blockhash transaction, so
the fee-payer rules of [SIMD-0290] apply to all of them. Rules that apply only
to DNTs, including any adopted under [SIMD-0297] or [SIMD-0602], have no effect.

Transaction validation, fee-only processing and failed-transaction rollback MUST
NOT read, advance or commit a nonce account because of the transaction's
lifetime. A failed transaction therefore commits at most its fee debit.

### Existing nonce accounts

Activation MUST NOT modify any nonce account. Nonce account fee-payer
eligibility and reserve requirements are unchanged.

All System Program nonce instructions MUST retain their existing behavior. In
transactions with a valid recent blockhash, they follow ordinary execution and
rollback rules.

### Block production

Leaders SHOULD drop DNTs using the ordinary recent-blockhash check, before
loading accounts for execution.

### Validator Components Affected

| Validator Component             | Impact                             |
|---------------------------------|------------------------------------|
| Transaction Execution (Runtime) | Lifetime and rollback              |
| Virtual Machine                 | None                               |
| Block Packing                   | Drop DNTs at the blockhash check   |
| Consensus                       | Blocks with a DNT are invalid      |
| Gossip                          | None                               |
| Turbine                         | None                               |
| Snapshots                       | None                               |
| On-Chain Core BPF Programs      | None                               |
| RPC                             | Preflight, simulation and retries  |

## Alternatives Considered

- Wrap existing DNTs in a fresh transaction, as in the Transaction Envelopes and
  Sudo Instruction designs from the [Nonce Replacement Meta-Proposal] or the
  [Nonce Payload Transaction Format] discussion. This keeps presigned DNTs
  usable and avoids moving assets to new authorities. We reject this alternative
  because it moves the special cases rather than removing them.
- Retain durable nonces and fix their problems as they appear. Existing
  leader-side filtering and the fee policy proposed in [SIMD-0571] address spam
  without requiring migration. We reject this alternative because this special
  casing has repeatedly caused the security and consensus issues described in
  the Motivation section.
- Retain durable nonces with the failure-handling changes in [SIMD-0297].
  Recording state-dependent nonce failures as unpaid no-ops would ease
  asynchronous execution. We reject this alternative because it retains nonce
  validation and cache coordination while allowing the same invalid transaction
  in multiple blocks.
- Also retire nonce account instructions and fee-payer eligibility. These are
  separate account-management rules and are left to a follow-up proposal.

## Impact

Any workflow that signs a transaction and submits it after the blockhash window
must change before activation. This includes custodians and exchanges that use
air-gapped signing, where approvals take longer than that window.

### Program-based replacements

Programs such as [Programmatic Signer] and [Blueshift Vector] are designed to
support offline and delayed signing without durable nonces. The owner first
rotates ownership and authorities to a PDA. The owner then signs an
authorization offline, which a relayer later submits in an ordinary transaction.
The program verifies the signature onchain and executes the approved
instructions through CPI. Its replay protection is ordinary program state, so
the protocol needs no special handling.

## Security Considerations

- A DNT signed before activation can never become valid after it, because
  lifetime then depends only on the blockhash queue.
- Risk moves to replacement programs, which must implement signature
  verification, replay protection, expiry, and revocation correctly. A failed
  execution can leave an authorization usable, so a failed submission is not a
  revocation.
- The proposal adds no new way to move funds. Withdrawals and authority changes
  require the same signatures as before.

[original-nonce-proposal]: https://github.com/solana-labs/solana/blob/8590326b509bafa839c7260f901d41d28068ad98/book/src/proposals/durable-tx-nonces.md
[solana#6725]: https://github.com/solana-labs/solana/pull/6725
[solana#6845]: https://github.com/solana-labs/solana/pull/6845
[solana#7443]: https://github.com/solana-labs/solana/issues/7443
[Neodyme]: https://www.neodyme.io/en/blog/nonce-upon-a-time/
[Solana report]: https://solana.com/news/06-01-22-solana-mainnet-beta-outage-report-2
[fd#1500]: https://github.com/firedancer-io/firedancer/pull/1500
[fd#9442]: https://github.com/firedancer-io/firedancer/pull/9442
[fd#9512]: https://github.com/firedancer-io/firedancer/pull/9512
[agave#9455]: https://github.com/anza-xyz/agave/pull/9455
[Jito release]: https://github.com/jito-foundation/jito-solana/releases/tag/v4.2.0-beta.2-jito.1
[agave#14123]: https://github.com/anza-xyz/agave/pull/14123
[agave#8970]: https://github.com/anza-xyz/agave/issues/8970
[Nonce Replacement Meta-Proposal]: https://github.com/solana-foundation/solana-improvement-documents/discussions/456
[Nonce Payload Transaction Format]: https://github.com/solana-foundation/solana-improvement-documents/discussions/415
[Programmatic Signer]: https://github.com/solana-program/ed25519-programmatic-signer
[Blueshift Vector]: https://github.com/blueshift-gg/vector
[SIMD-0242]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0242-static-nonce-account.md
[SIMD-0290]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0290-relax-fee-payer-constraint.md
[SIMD-0297]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0297-relax-invalid-nonce.md
[SIMD-0571]: https://github.com/solana-foundation/solana-improvement-documents/pull/571
[SIMD-0602]: https://github.com/solana-foundation/solana-improvement-documents/pull/602
