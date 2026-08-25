---
simd: '0610'
title: Prohibit Nonce Account Self-Withdrawals
authors:
  - Gabe Rodriguez (Anza)
category: Standard
type: Core
status: Review
created: 2026-08-25
feature: (to be assigned upon acceptance)
---

## Summary

Reject a System Program `WithdrawNonceAccount` instruction when the nonce account
and recipient resolve to the same address. After validating the instruction
accounts and nonce state, the System Program returns `InvalidArgument` when the
addresses match. Other nonce withdrawals are unchanged.

Originating discussion: [#598]

## Motivation

Currently, a System Program `WithdrawNonceAccount` instruction may use the same
account as the nonce account and recipient. Self-withdrawals from uninitialized
nonce accounts and zero or partial self-withdrawals from initialized nonce
accounts that pass existing checks subtract and then restore the requested
lamports, making them no-ops.

A full self-withdrawal from an initialized nonce account that passes the existing
checks behaves differently. It deinitializes the nonce account, subtracts its
full balance, and restores the lamports to the same account. This leaves a
funded, System Program-owned, uninitialized nonce account.

`InitializeNonceAccount` does not require the nonce account or new authority to
sign. Another party can initialize the account with themselves as the new nonce
authority and control the retained balance. Initiating the self-withdrawal
requires the existing nonce authority, making this an operator footgun.

Self-withdrawals have no known use case. The [Stake Program] rejects them with
`InvalidArgument`, and [SIMD-0609] proposes the same rule for Vote Program
withdrawals.

## New Terminology

N/A

## Detailed Design

System Program `WithdrawNonceAccount` processes an instruction in this order:

1. Validate the instruction account count and sysvars, require a writable source,
   and decode the nonce state.
2. Compare the addresses of the nonce account and recipient.
3. If the addresses match, return `InvalidArgument`.
4. If the addresses differ, continue existing withdrawal processing.

The new comparison runs before the initialized and uninitialized state branches
and their checks for balance, rent, nonce expiration, and required signers. It
therefore applies to zero, partial, full-balance, and over-balance withdrawals.

The instruction data and account interface do not change.

## Alternatives Considered

- Reject only full self-withdrawals from initialized nonce accounts. This
  prevents deinitialization but preserves other no-op self-withdrawals.
- Skip deinitialization for full self-withdrawals. This makes them no-ops and
  leaves a special case that future changes must continue to handle safely.
- Reject self-withdrawals only in the SDK and CLI. Custom transaction builders
  can bypass client-side checks, so this does not enforce the invariant.
- Keep the existing behavior. This preserves the footgun.

## Impact

After activation, nonce account self-withdrawals that reach the address check
fail with `InvalidArgument`. Non-self withdrawals, instruction encoding, and
account lists are unchanged.

Because the System Program is a native builtin, validator clients must implement
the same feature-gated behavior before activation.

## Security Considerations

N/A

[#598]: https://github.com/solana-foundation/solana-improvement-documents/discussions/598
[SIMD-0609]: https://github.com/solana-foundation/solana-improvement-documents/pull/609
[Stake Program]: https://github.com/solana-program/stake/pull/96
