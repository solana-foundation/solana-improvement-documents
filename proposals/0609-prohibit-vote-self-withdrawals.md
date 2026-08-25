---
simd: '0609'
title: Prohibit Vote Account Self-Withdrawals
authors:
  - Gabe Rodriguez (Anza)
category: Standard
type: Core
status: Review
created: 2026-08-25
feature: (to be assigned upon acceptance)
---

## Summary

Reject a Vote Program `Withdraw` instruction when the vote account and recipient
resolve to the same address. After activation, every authorized self-withdrawal
returns `InvalidArgument`. Other withdrawals are unchanged.

Originating discussion: [#594]

## Motivation

Currently, a Vote Program `Withdraw` instruction may use the same account as the
vote account and recipient. A partial self-withdrawal that passes existing checks
subtracts and then restores the requested lamports, making it a no-op.

A full self-withdrawal behaves differently. It deinitializes the vote account,
subtracts its full balance, and restores the lamports to the same account. This
leaves a funded, Vote Program owned, uninitialized account. Anyone else can
then initialize that account with their own validator identity and authorities,
taking control of the retained balance. Initiating the self-withdrawal requires
the existing authorized withdrawer, making the behavior an operator footgun.

Self-withdrawals have no known use case. Rejecting them removes this risk. The
[Stake Program] already rejects self-withdrawals with `InvalidArgument`.

## New Terminology

N/A

## Detailed Design

Vote Program `Withdraw` processes an instruction in this order:

1. Validate the vote account and authorized withdrawer using the existing rules.
2. Compare the addresses of the vote account and recipient.
3. If the addresses match, return `InvalidArgument`.
4. If the addresses differ, continue existing withdrawal processing.

The new comparison precedes withdrawal amount and account closure checks. It
therefore applies to zero, partial, full-balance, and over-balance withdrawals.

The instruction data and account interface do not change.

## Alternatives Considered

- Reject only full self-withdrawals. This prevents vote account deinitialization
  but preserves partial and zero-lamport no-op transactions without a known use.
- Skip deinitialization for full self-withdrawals. This makes them no-ops and
  leaves a special case that future changes must continue to handle safely.
- Reject self-withdrawals only in the SDK and CLI. Custom transaction builders
  can bypass client-side checks, so this does not enforce the invariant.
- Keep the existing behavior. This preserves the footgun.

## Impact

After activation, vote account self-withdrawals fail with `InvalidArgument`.
Non-self withdrawals, instruction encoding, and account lists are unchanged.

Because the Vote Program is a native builtin, validator clients must implement
the same feature-gated behavior before activation.

## Security Considerations

N/A

[#594]: https://github.com/solana-foundation/solana-improvement-documents/discussions/594
[Stake Program]: https://github.com/solana-program/stake/pull/96
