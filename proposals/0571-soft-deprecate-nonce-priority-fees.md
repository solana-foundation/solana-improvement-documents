---
simd: '0571'
title: Soft Deprecation of Durable Nonces
authors:
  - Max Resnick
category: Standard
type: Core
status: Idea
created: 2026-06-24
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Treat a durable nonce transaction's requested compute
unit price as `0` for both fee collection and scheduler prioritization. The
transaction remains valid and is not rejected; it simply cannot pay to be
prioritized, which removes the incentive to route fee-bidding traffic
through durable nonces.

## Motivation

Durable nonces are expected to be deprecated and eventually removed, but that
removal must wait for custodians and other non-time-sensitive users to
migrate. While durable nonce transactions still exist, they cause two
problems:

1. They are heavily used as a fee-bidding channel, generating high-volume
   retry traffic that congests the scheduler
2. Because that traffic pays competitive priority fees, it is coupled to the
   regular landing game and is expensive to ignore client-side alone.

Zeroing their priority fee removes the ability to outbid other traffic, and
acts as a forcing function: once durable nonces cannot be prioritized,
high-volume workloads have an incentive to migrate to the recent-blockhash
path ahead of full nonce removal.

Supporting data (last epoch, 10,000-block sample): the marginal non-vote,
non-Jito-bundle transaction paid a priority fee of `0`; durable nonce
transactions were ~3.1% of transactions but ~30% of priority-fee revenue
(1,975 / 6,621 SOL); 86% of them paid a priority fee versus ~28% network-wide;
and 0.5% of nonce fee-payers (555 of 103,672) drove 97.5% of nonce
transactions and 70% of nonce priority fees. The bulk of nonce priority-fee
revenue comes from a handful of high-volume bidding operations, not custodial
use — a rational response to the nonce path's favorable auction mechanics
(made precise below), but far from the custodial cold-signing use case the
mechanism exists to serve.

This spending on durable nonces is expected to mostly migrate over to
non-durable nonce transactions. Bidding through a durable nonce is closer to a
first-price auction (the priority fee is only paid on the attempt that lands),
while bidding through the standard path is closer to an all-pay auction
(failed attempts still land and pay). First-price bids are higher than all-pay
bids for the same opportunity; however, by the [revenue equivalence
theorem](https://en.wikipedia.org/wiki/Revenue_equivalence), the expected
total payment for the same underlying opportunity is the same across the two
formats: bidders shade their bids to offset the difference in payment rules.
Since this proposal does not change the size of the underlying opportunities,
expected spend should migrate to the standard path roughly one-for-one,
spread across more, lower-priced transactions.

## New Terminology

No new terminology. **Durable nonce transaction** is defined in Detailed
Design. **Prioritization fee** (priority fee) is the fee component derived
from the requested compute unit price and limit, and is distinct from the
per-signature base fee.

## Detailed Design

This change is gated behind a feature; the rules below apply to durable nonce
transactions once the feature is activated.

Durable nonce transactions have been defined more than once already: by the
shape of the first instruction (SIMD-0242), and by the runtime's full nonce
check over account state, with slight variations across implementations and
layers. This proposal attempts to resolve the inconsistencies by introducing
a new definition that should be treated as canonical going forward.

[![How standards proliferate (xkcd 927)][xkcd-img]][xkcd-927]

[xkcd-img]: https://imgs.xkcd.com/comics/standards.png
[xkcd-927]: https://xkcd.com/927/


> A transaction is a **durable nonce transaction** if and only if its
> `recent_blockhash` field is not a valid recent blockhash of the bank
> processing it.

This simple rule is sufficient because the account-dependent parts of the
runtime's existing nonce check gate validity, not classification. A
transaction whose `recent_blockhash` is not a valid recent blockhash can
only be committed by passing that check: its first instruction must advance
a System-owned, initialized nonce account whose stored durable nonce equals
the `recent_blockhash` field, signed by the nonce authority, with the nonce
still advanceable. If it fails the check, the transaction can never land and
never pays fees. So for every transaction that can land, "stale
`recent_blockhash`" and "processed via the durable nonce path" coincide.
This proposal does not modify the nonce check itself.

Conversely, a transaction with a valid recent blockhash is processed via the
normal path with normal fee and priority semantics — even if its first
instruction is `AdvanceNonceAccount`. Such a transaction can still advance
the nonce (and fails if the nonce was already advanced in that slot), but it
lands and pays fees like any other transaction. This is no different from
what any user-defined nonce program can already do, so it introduces no new
congestion or fee-evasion vector.

For a durable nonce transaction, after activation:

- Its prioritization fee MUST be computed as `0`, irrespective of the CU
  price requested in the message. The fee charged to
  the fee-payer therefore includes the base fee and any other
  non-prioritization components, but MUST NOT include a prioritization fee.
  This affects bank state and is consensus-critical: it MUST be applied
  identically by all validators under the feature gate, wherever the fee
  enters consensus-relevant computation — including fee-payer balance
  validation, pre-execution fee checks, and fee debit and collection. In
  particular, a fee-payer able to cover only the base fee (and other
  non-prioritization components) is valid after activation.
- For scheduling, a leader SHOULD treat the transaction's CU price as `0`
  when deriving scheduling priority. This is leader-local behavior and is not
  consensus-critical; applying it is what removes the incentive to bid
  through the nonce path.

Durable nonces remain valid and can land in blocks that have excess capacity.

## Alternatives Considered

- **Client-side only (no protocol change).** Rejected: a revenue-seeking
  client would fork to reverse it mid-migration, restoring the incentive to
  bid through the nonce path.

- **Burn the priority fee instead of not charging it.** Harder to implement.

- **Increase fees for durable nonce transactions instead.** Raising the base
  fee or the cost of `AdvanceNonceAccount` offers similar functionality in
  principle. It would encourage MEV-related traffic to migrate off of the
  durable nonce path; however, it is not compatible with bankless leader DoS
  prevention. A bankless leader admits and packs transactions without account
  state, so it cannot verify at admission time that a fee-payer can cover an
  elevated nonce-specific fee; underfunded durable nonce transactions could
  still consume block space while ultimately paying nothing. Zeroing the
  priority fee means that nonced transactions can never outbid normal
  transactions that pay at least some priority fee, which lines up nicely
  with the requirements for bankless. Note that increasing the fee does not
  do this, because what we are worried about is transactions that don't pay
  fees: increasing the fee for a transaction that does not pay the fee does
  nothing to deter this behavior.

## Impact

- **Legitimate durable nonce users (e.g. custodians):** mostly unaffected.
  The marginal priority fee in blocks is almost always `0` and ~94% of blocks
  are not full, so a priority-fee-`0` transaction lands within a slot or two
  with very high probability — even more so after the 100M CU block limit
  activation. End-to-end latency for a custodial durable-nonce flow is
  therefore, with extremely high confidence, dominated by the cold-signing
  operation itself, not by waiting for a non-full block.
- **High-volume bidding workloads:** lose the ability to prioritize durable
  nonce transactions, incentivizing migration to the standard path ahead of
  full removal.
- **Validators / leaders:** lose some fees in the short term,
  since the fee-payer is not charged (cf. SIMD-0096); however,
  this should mostly migrate to standard path since opportunity size has not
  changed.
- **Out-of-protocol consumers of compute unit price:** consumers that derive
  fees or priority from the CU price (RPC fee estimation, transaction
  simulation, packet admission or priority filtering above the scheduler)
  SHOULD use the effective CU price (`0` for durable nonce transactions).
  Transaction metadata / RPC SHOULD serve the priority fee and effective CU
  price as `0`, and SHOULD additionally expose the requested CU price as a
  new field, analogous to Ethereum serving `effectiveGasPrice` alongside the
  signed gas fields. The signed transaction data itself is unchanged.

## Security Considerations

- **Consensus determinism.** The fee charged to the fee-payer changes, so the
  fee-computation portion affects bank state and MUST be applied identically
  by all validators under the feature gate; divergence would cause a consensus
  failure.
- **Transaction classification.** Implementations MUST classify durable
  nonce transactions exactly by the recent-blockhash rule defined above.
  Classification depends only on the message and the blockhash queue, never
  on account state, so there is no load-order or state-divergence hazard.

## Backwards Compatibility

Not backwards compatible across activation, since the fee charged for a durable
nonce transaction changes; validators not running a client implementing this
feature MUST NOT be relied upon for consensus-compatible blocks afterward. The
transaction format is unchanged and already-signed durable nonce transactions
remain valid.
