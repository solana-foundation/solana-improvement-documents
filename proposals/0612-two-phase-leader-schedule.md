---
simd: '0612'
title: Two-Phase Leader Schedule
authors:
  - Jota
category: Standard
type: Core
status: Draft
created: 2026-08-26
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

The leader schedule is built from stake-weighted sampling **with replacement**:
once per leader rotation, a vote-keyed slot leader is drawn from a fixed stake
distribution using an epoch-seeded ChaCha RNG. With the default epoch length
and a 4-slot leader window that is **108,000 independent draws**, which
produces large per-epoch variance between expected and scheduled slot counts.

This SIMD replaces that sampler with a **two-phase count assignment** plus a
**separate shuffle** so that every staked vote account receives either
`floor(stake × num_rotations / total_stake)` or
`floor(stake × num_rotations / total_stake) + 1` rotations each epoch — i.e.
scheduled slots stay within **± one leader window** of the stake-proportional
share (today **±4 slots**). Counts are assigned first (deterministic floors,
then weighted remainder draws). Ordering is a Fisher–Yates permutation of the
rotation multiset, so leaders are not laid down in phase-1-then-phase-2 order.
Schedule generation remains deterministic and locally verifiable from public
stake and the epoch seed.

Originating discussion:
[Discussion #580](https://github.com/solana-foundation/solana-improvement-documents/discussions/580).

## Motivation

Leader slots drive block rewards and MEV income. Operators forecast epoch
revenue from stake, but under the current schedule that forecast has **large
per-epoch error**: variance from sampling noise, not from performance or stake
changes.

Ten consecutive epochs for one operator at **~0.056% stake** (~240K SOL):

| Epoch | Expected | Scheduled | Δ (slots) | Δ (%) |
| ---: | ---: | ---: | ---: | ---: |
| 827 | 239.7 | 232 | −7.7 | −3.2% |
| 826 | 244.1 | 240 | −4.1 | −1.7% |
| 825 | 251.3 | 224 | −27.3 | −10.9% |
| 824 | 257.6 | 284 | +26.4 | +10.2% |
| 823 | 256.1 | 228 | −28.1 | −11.0% |
| 822 | 255.4 | 296 | +40.6 | +15.9% |
| 821 | 255.7 | 224 | −31.7 | −12.4% |
| 820 | 244.4 | 260 | +15.6 | +6.4% |
| 819 | 242.4 | 264 | +21.6 | +8.9% |
| 818 | 242.3 | 300 | +57.7 | +23.8% |

Single-epoch deviations here reach on the order of **±58 slots**. The proposed
schedule keeps each vote account within **± one leader window** (±4 slots with
today's window size).

### Network-wide evidence (epoch 996)

Stake from epoch **995** (`activated_stake`); actual slots from epoch **996**
(`leader_slots`).

Expected slots = `(stake / total_stake) × 432,000`. The schedule for epoch
*N+1* uses the stake snapshot from epoch *N*, as today.

| Metric | Current schedule |
| --- | ---: |
| Vote accounts analyzed | 696 |
| Median \|Δ\| | **18.5 slots** |
| P95 \|Δ\| | **98.1 slots** |
| Max \|Δ\| | **334.6 slots** |
| Within ±4 slots | **15.1%** |
| Within ±8 slots | **24.9%** |

Under the proposed schedule, **every** staked vote account lands within ± one
leader window of its proportional share every epoch.

**Largest deviations (epoch 996):**

| Operator | Stake | Expected | Scheduled | Δ | Δ (%) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Galaxy | 6.3M | 6,323 | 5,988 | −335 | −5.3% |
| Bitwise | 8.1M | 8,199 | 8,496 | +297 | +3.6% |
| P2P.org | 4.1M | 4,142 | 3,896 | −246 | −5.9% |
| Mellow Yellow | 60K | 60 | 108 | +48 | +79% |
| Project 0 Meridian | 74K | 74 | 28 | −46 | −62% |

Expected outcome: stake remains the long-run determinant of leader share, while
per-epoch schedule noise shrinks to the unavoidable ±1-rotation band of
integer apportionment. Rotation order within the epoch is a random permutation
of the assigned multiset (Fisher–Yates).

## New Terminology

- **Rotation weight** — the exact quotient
  `rotation_weight = total_stake / num_rotations` (a rational in stake units).
  It is the stake corresponding to one leader rotation. Implementations MUST
  use integer-safe arithmetic equivalent to this exact quotient (see Detailed
  Design); they MUST NOT replace it with `floor(total_stake / num_rotations)`
  for floor or remainder calculation, which would break the floor invariant
  below.

- **Remainder weight** — a vote account's leftover weight after phase 1:
  `stake - (F_i × rotation_weight)`, where
  `F_i = floor(stake_i × num_rotations / total_stake)`.

- **Rotation multiset** — the list of slot leaders formed by repeating each
  vote account's `leader` entry once per assigned rotation (floor wins plus
  any phase-2 win). Before expansion to slots, this multiset is shuffled.

## Detailed Design

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC
2119](https://www.ietf.org/rfc/rfc2119.txt) and [RFC
8174](https://www.ietf.org/rfc/rfc8174.txt).

### Status Quo

Vote-keyed leader schedule construction works as follows:

1. Load staked vote accounts for the schedule epoch. Keep entries with
   `stake > 0`. Each entry becomes a slot leader
   `{ vote_address, id: node_pubkey }` with that stake.
2. Sort deterministically: **stake descending**, then **`vote_address`
   descending**; dedup equal `(stake, vote_address)` pairs.
3. Seed ChaCha from the epoch: 32-byte seed with `epoch.to_le_bytes()` in the
   first 8 bytes (remaining bytes zero).
4. Let `repeat = num_consecutive_leader_slots` (currently 4) and
   `num_rotations = slots_in_epoch / repeat` (108,000 with the default
   432,000-slot epoch). Each rotation is a contiguous run of `repeat` slots
   assigned to one slot leader. Build a **fixed** stake-weighted index over
   the stakes and sample **with replacement** once per rotation. Produce a
   compact rotation list (one slot leader per rotation); slot lookup expands
   each entry across `repeat` consecutive slots.

That is `num_rotations` independent draws with replacement. Expected slot
counts are stake-proportional; realized counts have binomial / multinomial
variance that grows with stake.

### Feature Gate

A feature gate `two_phase_leader_schedule` SHALL select how the
schedule is constructed at **schedule-generation time**:

- If the gate is **inactive** when a schedule is generated, clients MUST use
  the status-quo with-replacement sampler.
- If the gate is **active** when a schedule is generated, clients MUST use the
  two-phase count assignment and Fisher–Yates ordering below.

**Activation timing.** Leader schedules are generated ahead of the epoch they
govern. Implementations MUST apply the gate as follows:

1. Feature goes pending during epoch `N`.
2. Feature activates on the transition from epoch `N` to `N+1`.
3. The schedule for epoch `N+1` was already generated during epoch `N` with
   the with-replacement sampler and MUST NOT be regenerated.
4. The first schedule produced with the two-phase sampler is the schedule for
   epoch `N+2`, generated during epoch `N+1` while the gate is active.

All clients MUST agree on which sampler produced epoch `E`'s schedule from
this timing rule.

### Inputs Unchanged

Implementations MUST continue to use:

- The same **vote-keyed** stake snapshot as today: vote address →
  `(stake, vote_account)`, filtered to `stake > 0`, with
  `leader.id = vote_account.node_pubkey()`.
- The same deterministic stake sort (stake descending, then `vote_address`
  descending) and dedup rules.
- The same epoch-derived ChaCha seed and weighted-sample / RNG primitive
  family as today (used for phase-2 remainder draws and the Fisher–Yates
  shuffle).
- The same `slots_in_epoch` and `num_consecutive_leader_slots` / `repeat`
  used when generating that schedule.

Let:

- `entries = [(leader_i, stake_i)]` be the sorted vote-keyed leaders with
  `stake_i > 0`, where `leader_i = { id_i, vote_address_i }`
- `total_stake = sum(stake_i)`
- `num_rotations = slots_in_epoch / num_consecutive_leader_slots`
- `rotation_weight = total_stake / num_rotations` (exact quotient)

If `total_stake == 0` or `num_rotations == 0`, behavior is unchanged from
today (schedule generation is already undefined / unused in those cases).

**Floor invariant.** With this exact `rotation_weight`,

`sum_i floor(stake_i × num_rotations / total_stake) ≤ num_rotations`.

Phase 1 therefore always finishes issuing every vote account's floor with
`sum(F_i) ≤ num_rotations`; it cannot overrun the rotation budget. Phase 2
fills `num_rotations - sum(F_i)` remainder rotations.

### Phase 1: Deterministic Floors

Phase 1 uses **no randomness**. For each vote account `i` in `entries` order:

1. Compute `F_i = floor(stake_i × num_rotations / total_stake)`.
2. Record that `i` is owed `F_i` rotations.

Equivalently, in the integer-safe scaled form below,
`F_i = floor(scaled_i / total_stake)` with `scaled_i = stake_i × num_rotations`.

When phase 1 completes, each vote account has been assigned exactly its
deterministic floor `F_i`, and `sum(F_i) ≤ num_rotations`.

**Integer-safe form (normative).** To avoid floats and avoid flooring
`rotation_weight`, implementations MUST use an equivalent scaled
representation, for example:

- `scaled_i = stake_i × num_rotations` initially
- `F_i = floor(scaled_i / total_stake)` (integer division)
- after assigning the floor: `remainder_scaled_i = scaled_i - F_i × total_stake`
  (equivalently `scaled_i % total_stake`)

This yields the same floors and remainder weights as exact rational
`rotation_weight`, with
`F_i = floor(stake_i × num_rotations / total_stake)`.

### Phase 2: Remainder Draw Without Replacement

Let `remaining_rotations = num_rotations - sum(F_i)`.

Let `count_i = F_i` for each vote account initially. Let
`remainder_weight_i` be the leftover from phase 1 (equivalently
`stake_i - F_i × rotation_weight`, or `remainder_scaled_i` in the
integer-safe form).

If `remaining_rotations > 0`:

1. Repeat `remaining_rotations` times:
   - Sample among entries with `remainder_weight_i > 0`, weighted by
     `remainder_weight_i`, visiting candidates in the same deterministic
     `entries` order (stake descending, then `vote_address` descending).
   - Increment the winner's `count_winner` by 1.
   - Set that winner's `remainder_weight_i = 0` (at most one phase-2 win per
     vote account).

Vote accounts with `stake_i < rotation_weight` receive zero phase-1 wins and
compete in phase 2 with remainder weight equal to their full stake.

After both phases, each vote account's rotation count MUST be either `F_i` or
`F_i + 1`, where `F_i = floor(stake_i × num_rotations / total_stake)`, and
`sum(count_i) == num_rotations`.

### Ordering: Fisher–Yates Shuffle

Count assignment alone must not dictate schedule order. An earlier design wrote
rotations in assignment order (all floors, then remainder winners). Remainder
winners are disproportionately smaller validators, who also tend to have
**higher skip rates**. Parking them in a contiguous **tail** of the epoch
concentrates skip risk and can amplify fork pressure.

After counts are fixed, implementations MUST:

1. Build the **rotation multiset**: for each `i` in `entries` order, append
   `leader_i` exactly `count_i` times.
2. **Randomly permute** that multiset with **Fisher–Yates**, consuming the
   epoch-seeded ChaCha RNG (same primitive family as today's weighted sample).
   The shuffle MUST run after all phase-2 samples, so RNG consumption order is:
   phase-2 draws first, then Fisher–Yates swaps.
3. The resulting list is `assigned` — exactly `num_rotations` slot leaders.

This preserves the floor / floor+1 count bound while eliminating positional
bias: phase-2-only validators are not systematically scheduled at the end of
the epoch.

### Schedule Construction

`assigned` is the compact rotation list after the shuffle. For rotation index
`r`, slots
`[r * num_consecutive_leader_slots, (r + 1) * num_consecutive_leader_slots)`
are led by `assigned[r]` (same `vote_address` and `id`).

This SIMD constrains **counts** to the floor / floor+1 band and requires a
uniform random permutation of the rotation multiset for **order** within the
epoch.

### Worked Example

Default epoch: 432,000 slots ⇒ 108,000 rotations with a 4-slot window.
Vote account at **~0.056%** stake (~240K SOL); `rotation_weight ≈ 3,966 SOL`:

- Expected rotations ≈ **60.5**
- Phase 1 assigns **60** (deterministic floor)
- Phase 2: leftover ~2K competes for the fractional share → **0 or 1** more
  rotation
- Final: **60 or 61 rotations** → **240 or 244 slots** (±4); Fisher–Yates
  places those rotations across the epoch

### Pseudocode

```text
fn generate_leader_rotations(entries, slots_in_epoch, window, seed):
    # entries: [({ id, vote_address }, stake)],
    # sorted stake desc, then vote_address desc
    num_rotations = slots_in_epoch / window
    total_stake = sum(stake for (_, stake) in entries)
    # Integer-safe exact rotation_weight = total_stake / num_rotations:
    scaled = [stake * num_rotations for (_, stake) in entries]
    counts = [0; len(entries)]
    remainder = [0; len(entries)]
    rng = ChaChaRng(seed)

    # Phase 1 — deterministic floors; sum of floors <= num_rotations always
    for i in entries.indices:
        counts[i] = scaled[i] / total_stake          # integer division
        remainder[i] = scaled[i] % total_stake

    # Phase 2 — remainders without replacement
    remaining = num_rotations - sum(counts)
    for _ in 1..=remaining:
        positive = [i for i in entries.indices if remainder[i] > 0]
        # positive preserves entries order
        winner = weighted_sample(positive, weights=remainder, rng)
        counts[winner] += 1
        remainder[winner] = 0

    # Build rotation multiset in entries order
    assigned = []
    for i in entries.indices:
        for _ in 1..=counts[i]:
            assigned.append(entries[i].leader)  # { id, vote_address }

    # Order — Fisher–Yates shuffle (same ChaCha stream)
    for i in (len(assigned) - 1) downto 1:
        j = rng.gen_range(0..=i)   # uniform in [0, i]
        swap(assigned[i], assigned[j])

    assert len(assigned) == num_rotations
    assert sum(counts) == num_rotations
    return assigned
```

### Edge Cases

1. **Vote accounts with stake `< rotation_weight`.** Floor is 0; they appear
   only in phase 2, still at most one extra rotation.

2. **Equal stakes.** Sort-by-`vote_address` (descending) tie-break and RNG
   (phase 2 and shuffle) decide remainder wins and order; counts still land
   in the floor / floor+1 band.

3. **Exact multiples of `rotation_weight`.** `F_i` consumes the full scaled
   stake; remainder is 0, so the account does not compete in phase 2 and
   receives exactly its floor.

### Validator Components Affected

| Validator Component | Impact |
| --- | --- |
| Transaction Execution (Runtime) | None directly |
| Virtual Machine | None |
| Block Packing | None |
| Consensus | Clients MUST compute the same schedule or the cluster forks |
| Gossip | None |
| Turbine | None (consumes schedule only) |
| Snapshots | None |
| On-Chain Core BPF Programs | None |
| Other | Leader schedule generation and offline schedule regenerators |

## Alternatives Considered

**Keep with-replacement sampling.** Rejected. Stake-proportional in expectation,
but per-epoch errors of tens to hundreds of slots are large relative to
operator forecasting and to the ±1-rotation bound achievable with
apportionment.

**Hamilton / largest-remainder (deterministic remainders).** Assign every vote
account its quota floor, then give remaining rotations to the largest
fractional remainders with a deterministic tie-break. Rejected: a deterministic
remainder ranking is stake-grinding friendly — operators can nudge stake to
maximize their fractional part and reliably claim leftover rotations. This SIMD
keeps weighted ChaCha sampling for remainder selection (phase 2) so fractional
shares compete proportionally to leftover stake without a fixed ranking to
optimize against.

**Write schedule in phase-1-then-phase-2 assignment order (no shuffle).**
Rejected. Remainder winners skew toward smaller validators with higher skip
rates; a contiguous epoch tail of those leaders concentrates skip risk.
Fisher–Yates after counts removes that positional bias while preserving the
count bound.

**Single-phase depleting draw until `num_rotations`.** Rejected. Once weights
fall below `rotation_weight`, the sampler can pile extra rotations onto small
remainders and skip vote accounts that still deserve another full rotation,
breaking the floor / floor+1 guarantee. Deterministic floors plus a separate
remainder phase avoid that failure mode.

## Impact

- **Validators / operators:** Epoch slot counts become forecastable from stake
  within ± one leader window. Revenue variance from sampling noise drops
  sharply; stake changes and skip rate remain the dominant drivers. Rotation
  positions within the epoch remain randomly distributed.
- **dApp developers:** No API or transaction format change. Any logic that
  assumed highly variable per-epoch leader counts for a given stake SHOULD
  expect tighter bands after activation.
- **Core contributors:** Replace the schedule sampler behind a feature gate;
  add cross-client conformance fixtures. No change to replay, banking, or VM
  beyond consuming the new schedule. Vote-keyed `{ id, vote_address }` entries
  are unchanged.

## Security Considerations

This change is **consensus-breaking**: every client MUST compute the identical
`assigned` list of slot leaders for a given vote-keyed stake vector, window
size, and epoch seed after activation, or the cluster forks on scheduled
`vote_address` / block-signer `id`.

Schedule generation MUST remain locally verifiable from public stake and the
epoch seed. No new trusted party is introduced.

Phase-2 weighted sampling MUST use the same stake sort (including
`vote_address` tie-break), the same positive-set iteration order, and the same
RNG consumption order on all clients. The Fisher–Yates shuffle MUST use the
same ChaCha stream **after** phase-2 samples, with a bit-identical
`gen_range(0..=i)` (or equivalent) for each swap. Floating-point weights MUST
NOT be used; integer stake / scaled weights MUST be used.

Phases 1–2 and the shuffle do not change which vote accounts can lead — only
the distribution of rotations among already-staked vote accounts and their
order within the epoch. Stake grinding attacks remain bounded by the cost of
stake as today; reducing schedule-count variance removes a noise term but does
not create a new grinding surface beyond what stake-weighted leadership already
implies. The shuffle avoids concentrating higher-skip leaders at a fixed
region of the epoch.

## Drawbacks

- Slightly more complex schedule generation than a single weighted-index loop
  (floor pass, remainder draws, then Fisher–Yates).

## Backwards Compatibility

- **Before activation:** unchanged with-replacement schedules.
- **After activation:** per the timing rule above, epoch `N+1` still uses the
  pre-generated with-replacement schedule; epoch `N+2` is the first epoch
  whose schedule uses the two-phase sampler. Nodes without the gate
  implementation WILL diverge when that schedule takes effect.
- No RPC encoding, account, or transaction format changes. The schedule remains
  vote-keyed.
- Historical epochs MUST keep using the sampler that was effective when those
  schedules were produced (feature-gate replay semantics as with other
  schedule-affecting gates).

## Conformance

Client implementations MUST include tests or fixtures that demonstrate:

1. **Bit-identical schedules** across implementations for fixed
   `(entries, epoch_seed, slots_in_epoch, window)` inputs after the feature is
   active, including matching `vote_address` and `id` on every slot.
2. **Count bound:** each vote account's rotation count is `F_i` or `F_i + 1`
   with `F_i = floor(stake_i × num_rotations / total_stake)`, and
   `sum(counts) == num_rotations`.
3. **Expansion:** each rotation expands to exactly `window` consecutive slots
   for the same slot leader.
4. **Legacy path:** with the feature inactive, schedules match today's
   with-replacement sampler.
5. **Edge fixtures:** many sub-`rotation_weight` vote accounts; stakes that are
   exact multiples of `rotation_weight` (full floor, zero remainder).
6. **Activation boundary:** a localnet or ledger fixture covering epochs `N`
   through `N+2` as in Feature Gate timing — schedule for `N+1` still
   with-replacement, schedule for `N+2` two-phase — with all nodes agreeing on
   slot leaders across that boundary.
7. **RNG order:** fixtures that pin phase-2 sample outcomes and the subsequent
   Fisher–Yates permutation for a fixed seed, so clients agree on stream
   consumption.
