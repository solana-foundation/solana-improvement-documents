---
simd: '0538'
title: Raise Block Limit to 1B CUs Under Alpenglow
authors:
  - Igor Durovic (Anza)
category: Standard
type: Core
status: Idea
created: 2026-05-12
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

After Alpenglow is activated, raise the `Max Block Units` limit to `1_000_000_000`
compute units so that the block CU limit no longer acts as the primary bound on
replay time. Instead, replay time is bounded by Alpenglow timeouts and the
resulting skip behavior. This proposal also depends on
[SIMD-0539](https://github.com/solana-foundation/solana-improvement-documents/pull/539),
which introduces a latent feature gate capable of restoring the legacy block CU
limit if needed.

## Dependencies

This proposal depends on the following accepted proposal:

- **[SIMD-0326]: [Alpenglow](https://github.com/solana-foundation/solana-improvement-documents/pull/326)**

  Alpenglow introduces timeout-based skip behavior, which becomes the primary
  protocol-level bound on replay time.

This proposal also depends on:

- **[SIMD-0539]: [Legacy Block CU Limit Safeguard](https://github.com/solana-foundation/solana-improvement-documents/pull/539)**

  This SIMD introduces a latent feature gate that, when activated, reduces
  `Max Block Units` back to the legacy level.

## Motivation

Before Alpenglow, the block CU limit is an important protocol-level bound on
how much execution work a leader can place into a block. That bound helps limit
replay time and reduces the chance that validators fall behind while processing
heavy blocks.

Under Alpenglow, replay time can instead be bounded by the protocol's local
timeout behavior. Validators that do not receive and replay a block quickly
enough will vote to skip the slot. That means the block CU limit is no longer
the only, or even the most natural, mechanism for bounding replay time at the
protocol level.

This proposal updates the design accordingly. Rather than fully removing the
block CU limit, it raises the limit to `1B` CUs so the limit remains present as
an implementation and safety hook, but is no longer expected to be the
practical bottleneck during normal operation.

## New Terminology

N/A

## Detailed Design

After the consensus migration is complete and Alpenglow is active, set:

| Type | Current Limit | Proposed Limit |
|------|---------------|----------------|
| Max Block Units | Legacy value | 1_000_000_000 |

All other block-level limits remain unchanged.

This proposal changes the role of `Max Block Units`:

- Before activation, `Max Block Units` is a meaningful protocol-level bound on
  execution work per block.
- After activation, `Max Block Units` remains in place but is set high enough
  that replay time is expected to be bounded by Alpenglow timeout behavior
  rather than by the CU ceiling.

Activation requirements:

- This SIMD must not activate before Alpenglow is active on the network.
- This SIMD should not activate until the latent safeguard feature gate from
  SIMD-0539 is implemented and shipped in validator clients.

Safeguard requirements:

- SIMD-0539 defines a latent feature gate that restores `Max Block Units` to
  the legacy value.
- The gate is expected to remain inactive during normal operation.
- If network conditions deteriorate, the gate provides a protocol-level path to
  reintroduce the legacy replay bound without requiring a new emergency SIMD.

## Alternatives Considered

- Remove the block CU limit entirely

  Keeping a very large limit is operationally safer than removing the mechanism
  outright. It preserves a clear rollback path and avoids turning "unbounded"
  execution into consensus behavior.

- Remove CU tracking and enforcement entirely

  With the current proposal, transaction-level CU tracking and budget
  enforcement is preserved, but removing CUs from the protocol entirely is also
  possible. This would be far more comprehensive, risky, and disruptive, with
  the main benefit being increased flexibility during block production. The
  potential for significant DoS vulnerabilities likely outweighs any benefits.

## Impact

- In normal operation, block packing is no longer expected to be constrained by
  `Max Block Units`.
- Replay time becomes primarily bounded by Alpenglow timeout behavior.
- Validator clients and operators will need stronger local block production
  safeguards because the protocol-level CU cap is no longer the main defense
  against leader skips.

## Security Considerations

- Risks:
  - Higher skip rate if blocks take too long to replay before local Alpenglow
    timeouts expire.
  - Greater geographic and hardware centralization pressure if some leaders can
    reliably pack and transmit larger blocks to enough nearby stake to obtain
    notarization, while more distant or less-provisioned validators fall
    behind.
  - Larger denial-of-service surface from valid but expensive workloads that
    increase replay lag or operational stress.
  - Similar in spirit to slot stretching, leaders may capitalize on the new
    option to make risky bets on late/large blocks. e.g. if a block packing
    strategy has a 10% skip chance but increases per block revenue by 15%, a
    profit maximizing leader will adopt it.  

- Safeguard:
  - The latent feature gate provides a pre-shipped path to restore the legacy
    block CU limit.
  - This reduces response time if elevated skip rate, replay instability, or
    centralization pressure is observed after activation.

- Local leader protections:
  - Because the block CU limit is no longer the primary protocol-level defense
    against leader skips, block production should implement local safeguards.
  - Examples include reducing the block production timeout when skip rate
    rises, locally falling back to the legacy block CU limit during periods of
    elevated skip rate, etc.

- Replay behavior:
  - A local skip vote does not imply that the slot will be skipped by the rest
    of the cluster.
  - Terminating replay immediately after voting skip can be counterproductive:
    if the block is later accepted, restarting execution from the beginning can
    leave the validator further behind than continuing execution.
  - This risk is higher for validators farther from the stake-weighted
    geographic center of the cluster.
  - Validator clients should support replay pausing and resumption so partially
    executed banks and associated block data can be retained and resumed.
  - Even with replay pausing supported, pausing immediately after issuing a skip
    vote may still be risky, as it can further delay a validator that is already
    running behind.
    - It is likely safer to pause only after observing stronger cluster evidence
      that the slot will be skipped, such as a valid skip certificate.

## Backwards Compatibility

All previously valid blocks remain valid.

Blocks produced after activation may be rejected by older software that does
not support the higher `Max Block Units` limit or the associated activation
logic.
