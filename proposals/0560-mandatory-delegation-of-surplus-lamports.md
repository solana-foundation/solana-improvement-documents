---
simd: '0560'
title: Mandatory Delegation of Surplus Lamports
authors:
  - Jarett Dunn (Stacc)
category: Standard
type: Core
status: Idea
created: 2026-06-10
feature: TBD (simd_0560_mandatory_delegation)
---

## Summary

Every lamport above an account's rent-exempt minimum is delegated. At
feature-gate activation, and thereafter at every lamport credit, the runtime
converts an account's surplus lamports into an approved liquid staking
position held against the account, and unwinds that position automatically on
transfer-out and on account close. Holding undelegated SOL above the rent
floor becomes a state the protocol no longer represents.

This proposal is submitted in the expectation of rejection. Its success
metric is stated in the *Status and success criteria* section, and it is not
feature-gate activation.

## Motivation

The protocol has spent the last several SIMD cycles deciding, piecemeal, what
idle SOL is for:

- **SIMD-0436** halved `lamports_per_byte` to 3,480. In one parameter change it
  reclassified roughly half of every existing rent balance as surplus - and a
  large share of that surplus is authority-orphaned: nobody holds a key that can
  claim it. The protocol minted, by fiat, a category of idle SOL with no live
  claimant. This proposal sweeps the category the protocol created.
- **SIMD-0389** (supervisory controller) was pitched, in its own advocacy, as
  "releasing billions of dormant SOL." That is the sincere version of this
  proposal. This document is the same claim with the euphemism removed.
- **SIMD-0490** (stake program v5) sets a 1 SOL minimum delegation - an explicit
  admission that small idle balances are beneath the protocol's attention. This
  proposal takes the opposite position: no balance is beneath the protocol's
  attention.
- The compression and dynamic-rent SIMDs - which transform *resting state* at
  access boundaries - are parked for complexity. This proposal is their
  value-side twin: it transforms *resting value* at access boundaries, and it
  inherits their core insight that the cheapest moment to act on dormant
  state is the moment it is touched.

The empirical pile is not hypothetical:

- **Pre-bond launchpad reserves.** pump.fun bonding-curve PDAs hold their SOL as
  native lamports. The last public measurement (November 2024, ~4M tokens
  launched at the time) found **~162,789 SOL** locked in never-graduated
  curves; with cumulative launches several times that today, the working
  estimate is **280-400K SOL**, on the order of 90% of it stagnant - attached
  to curves with `complete = false` and no live claimant - and the category is
  near-terminal as new flow moves to USDC curves. (Exact-figure methodology:
  Appendix B.)
- **The wSOL float.** `getTokenSupply` on the native mint
  (`So11111111111111111111111111111111111111112`) returns **0**. Third-party
  indexers count what the protocol will not: **22,403,599 wSOL across
  6,880,660 holder accounts** (≈$1.44B at time of writing). The largest
  deliberately idle SOL position on the network is invisible to the ledger
  that hosts it. (Treatment of wSOL: Appendix A.)
- **Orphaned rent surplus** (per SIMD-0436, above): unmeasured, but structurally
  guaranteed to be large and growing, since every historical rent deposit was
  sized at the old rate.

Idle SOL is diluted by emissions every epoch. The protocol's current answer is
that this is each holder's own business. This proposal forces the ecosystem to
say that out loud, with reasons, on the record.

## New Terminology

- **Surplus lamports**:
  `lamports(account) - rent_exempt_minimum(data_len(account))`, floored at
  zero.
- **Approved delegation target (ADT)**: a liquid staking pool admitted to the
  registry (below) and therefore eligible to receive forced flow.
- **Registry**: the feature-gate-governed list of ADTs.
- **Shadow position**: the runtime-managed record `(address → ADT, pool-token
  amount)` representing an account's swept surplus. Not a token account - the
  position must not itself create rent obligations, or the sweep recurses.
- **Sweep**: conversion of surplus lamports into a shadow position.
- **Unwind**: redemption of a shadow position back into lamports.
- **Credit boundary**: any instruction effect that increases an account's
  lamports.

## Detailed Design

### Sweep

1. **At activation**: every account's surplus is swept. Performed eagerly
   this is the largest single state transition in the chain's history (every
   account is touched), so it is performed *lazily*: an account is swept on
   first access after activation. The compression rhyme made literal -
   dormant value is transformed at the access boundary.
2. **After activation**: every credit boundary triggers a sweep of the credited
   account's surplus, in the same transaction, after the crediting instruction's
   effects settle.
3. Conversion occurs at the ADT's current redemption rate. ADT selection for new
   sweeps is pro-rata over registry stake weight (deliberately: see *Impact* -
   this is why the proposal moves no decentralization metric).

### Unwind

4. Debits are served first from the free rent-floor balance, then by redeeming
   the shadow position in-line. Account close redeems the entire position.
5. In-line redemption requires instant liquidity, so the runtime enforces a
   minimum unstaked **reserve ratio R** on every ADT. Note what this does: the
   idle SOL has not been eliminated, it has been relocated into the ADTs,
   renamed "reserve," and made load-bearing for native transfer liveness. The
   proposal concedes this openly (Objection 2).
6. Redemptions exceeding aggregate reserves fall back to the stake deactivation
   queue, i.e. the affected transfer **fails until a future epoch**. Native SOL
   transfers acquire, for the first time, a liveness dependency on third-party
   pool solvency.

### Mechanism variants

- **Variant A - runtime CPI.** The runtime invokes the ADT's program directly.
  This inverts the layering of the entire system: consensus calls up into
  upgradeable userspace code, and an ADT upgrade key becomes a consensus-level
  actor (Objection 1).
- **Variant B - enshrined adapter.** A native delegation interface with fixed
  semantics; ADTs must deploy an immutable adapter implementing it. This
  converts the layering inversion into an enshrined cartel with extra steps:
  the registry still picks winners (Objection 1, unimproved).
- **Variant C - the honest version.** Drop pools entirely and enshrine
  **rebasing**: all balances accrue stake yield pro-rata at epoch boundaries.
  No registry, no CPI, no reserve. This is what the proposal actually is once
  the indirection is removed. Its precedent (Blast) worked under a single
  sequencer; at L1 it couples consensus faults directly to everyone's money
  (Objection 4).

The author's position: anyone rejecting Variants A and B but unwilling to argue
against C in writing has conceded the motivation and is objecting to the
plumbing.

### Stake-warmup accounting

Forced flow is either exempt from activation/deactivation warmup or it is not.
If exempt, the warmup invariants that protect leader-schedule stability no
longer hold for an unbounded fraction of total stake. If not exempt, unwinds
stall up to a full epoch and clause 6 becomes the common case rather than the
edge case. There is no third option; the proposal declines to pretend otherwise.

### Feature gate

`simd_0560_mandatory_delegation`. Single gate; the lazy sweep makes activation
itself cheap and smears the cost over subsequent access patterns.

## Anticipated Objections

Stated here at full strength, because they are the deliverable.

1. **Layering inversion / enshrined cartel.** The runtime must not call
   upgradeable programs (Variant A), and a protocol-blessed LST whitelist is an
   enshrined delegation cartel whatever the variant (A or B). The registry's
   admission process becomes the most valuable political object on the network.
2. **The float is conserved, only relocated.** Instant unwind requires unstaked
   reserves; reserves are idle SOL by another name; and native transfers acquire
   bank-run dynamics - a correlated redemption wave converts a payments system
   into a queue. The proposal does not eliminate idle SOL; it launders it
   through pools and makes it systemically critical on the way.
3. **Yield → 0 at forced full participation.** Staking yield is an inflation
   transfer from non-stakers to stakers. At 100% participation everyone dilutes
   everyone identically and the real yield of the forced position converges to
   zero (less pool fees, plus whatever MEV/priority-fee flow distributes). The
   proposal forces the entire network to run in place - and, in passing,
   destroys the carry that funds the existing LST ecosystem, including the
   author's own (stacSOL). The author is proposing against book and considers
   this the proposal's strongest credential.
4. **The honest version is enshrined rebasing.** See Variant C. Rebasing at L1
   means a consensus bug is a balance bug for every account simultaneously. The
   one production precedent operated under a single sequencer with social
   recourse; an L1 has neither.

## Alternatives Considered

The salvageable shapes, in descending order of seriousness:

1. **Inactivity-gated sweep of terminal program state.** Sweep only PDAs in
   provably dead states - e.g. launchpad curves with `complete = false` and no
   credit boundary in N epochs. This is the present proposal minus one
   predicate, it captures the most offensive tranche of the empirical pile
   (≈300K SOL), and the Seoul Labs analysis converged on it independently. If
   this SIMD produces one durable artifact, it should be this.
2. **Stake the aggregate rent float.** The protocol stakes the sum of all
   rent-exempt minimums as a single protocol-owned position; no per-account
   semantics change, no custody change, yield accrues to (pick one: burn,
   validator subsidy, the foundation's grant budget). Small, boring, probably
   actually viable.
3. **The inverse: make staked SOL spendable.** Rather than forcing idle SOL to
   stake, let stake accounts serve as payment sources with
   deactivation-on-demand. Same end-state (the idle/staked distinction
   dissolves), opt-in, no custody seizure. Harder engineering, better politics.
4. **The app layer already shipped the opt-in version.** The YAL router is this
   proposal with consent. Its adoption rate is therefore *data*: it measures
   how much users actually value the optionality of idle SOL when offered the
   trade voluntarily. The delta between that number and 100% is the size of the
   claim this SIMD makes on other people's preferences.

## Impact

- **Decentralization: none.** Pro-rata flow into the existing stake distribution
  moves the Nakamoto coefficient by zero. The break-even validator count -
  roughly 3 today, an estimated 20-35 post-Alpenglow as vote costs fall - is
  improved by Alpenglow, not by this proposal. Forced delegation produces no
  decentralization yield; it is redistribution from non-stakers to the
  already-staked topology.
- **DeFi**: every protocol holding native lamports (AMM vaults, escrows,
  launchpad PDAs) becomes an involuntary staker. PDA-owned surplus is swept
  with no authority able to opt out - which is precisely the point of the
  proposal and precisely the problem with it.
- **Wallet/UX**: "balance" bifurcates into rent floor + shadow position;
  every balance display, balance check, and flash-repayment invariant on the
  network is wrong on activation day.

## Security Considerations

- Variant A makes an ADT program bug a ledger-wide balance-corruption event and
  an ADT upgrade key a consensus-level attack surface.
- Reserve depletion (clause 6) converts market stress into transfer-liveness
  failure: the bank run arrives through `SystemProgram::transfer`.
- A mass unwind cascading into the deactivation queue interacts with
  leader-schedule stake stability in ways this document does not pretend to have
  modeled.
- Variant C couples consensus faults to money at L1 with no recourse layer.

## Drawbacks

This section is the rest of the document.

## Backwards Compatibility

None worth claiming. Every program that reads lamports as spendable balance is
incorrect after activation. The migration burden falls on the entire deployed
program set simultaneously, which is not a migration plan; it is a re-genesis
with sentiment attached.

## Status and Success Criteria

This SIMD is performance art in the precise sense: it is a maximalist gesture
whose value is realized in being answered. The protocol currently holds an
implicit position - that idle SOL is sacred, that the rent floor is the only
balance the runtime may opine on - and that position has never been written
down, because nothing has ever forced anyone to write it down.

This proposal is the forcing function. It succeeds when the rejection rationale
is articulated at the same level of rigor as the objections enumerated above -
at which point the ecosystem will have produced, for the first time, a
normative document on what idle SOL is *for*. It fails only if it is ignored.

Reviewers are accordingly asked to reject it *well*.

## Appendix A - Extension to Wrapped SOL (severable)

The wSOL float is plausibly the largest deliberately idle SOL pile in
existence, and it is structurally uncounted: the native mint's supply field
reads zero, because wSOL is minted by lamport transfer plus `SyncNative` and no
ledger entry aggregates it. A complete version of this proposal sweeps it.

It is included here as a severable appendix rather than in the normative body,
deliberately:

- Sweeping token-account lamports breaks the token program's invariant
  `lamports == amount + rent_exemption` for the native mint, which converts
  every wSOL-handling program - every AMM, every router - into undefined
  behavior in one feature gate.
- Unwind-on-`CloseAccount`/`SyncNative` is mechanically identical to the
  transfer-out unwind in the body, so the extension adds no new machinery, only
  a new blast radius.
- Strategically: the body of this proposal should die on **economics**
  (Objections 2 and 3), which is the interesting death that produces the
  normative document. With wSOL in the normative body it would die on a
  **trivial invariant break**, which is the boring death that produces a
  one-line rejection.

The appendix therefore exists so that the question "and why is *wrapped* idle
SOL sacred too?" is formally on the record, while remaining detachable the
moment it threatens to give reviewers an easy out.

## Appendix B - Open Empirical Items

1. **Exact pre-bond reserve figure.** Public anchor: ~162,789 SOL measured
   November 2024 (Cryptopolitan/CryptoRank), when cumulative launches stood at
   ~4M tokens. Two refresh paths:
   - *Soft (Dune)*: join `pumpdotfun_solana.pump_evt_createevent.bonding_curve`
     against `solana_utils.latest_balances`, anti-join
     `pump_evt_completeevent`, sum `sol_balance` bucketed by curve age and
     `quote_mint` (the USDC-curve migration shows up directly).
   - *Hard (RPC)*: paginated `getProgramAccountsV2` on the pump program,
     `BondingCurve` discriminator `[23, 183, 248, 55, 96, 216, 172, 96]`,
     `dataSlice { offset: 32, length: 17 }` → `real_sol_reserves: u64 LE`
     (bytes 0-8) and `complete: bool` (byte 16), summed over
     `complete == false`.
2. **Candy-machine rent float.** Count and aggregate rent balance of dormant
   candy machine v2/v3 accounts - the historical analogue of item 1 and a
   second exhibit for the "authority-orphaned surplus" category.
3. **Orphaned post-0436 surplus.** Total rent balances above the new
   rent-exempt minimum on accounts with no authority activity since the rate
   change.
