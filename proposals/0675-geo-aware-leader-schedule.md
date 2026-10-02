---
simd: '0675'
title: Geo-Aware Leader Schedule
authors:
  - Quentin Kniep
  - Roger Wattenhofer
category: Standard
type: Core
status: Draft
created: 2026-09-29
feature: (fill in with feature key and github tracking issues once accepted)
extends: SIMD-0180, SIMD-0630
---

## Summary

This proposal changes how the epoch leader schedule is calculated. We are
adding a post-processing step after generating the initial stake-weighted
random schedule. In this post-processing, we change the order of the leaders
by carefully moving geographically close leaders next to each other. We need
to be careful because we must respect various security properties. The
post-processing step only reorders the leaders, every validator still
receives exactly as many slots as before. We use the self-reported
geo-locations of validators, as introduced in SIMD-0674.

## Motivation

Consider Alpenglow's fast leader handover with leaders sending their blocks
directly to the next leader. In the happy path of the handover, the delay
between two consecutive leaders is dominated by the one-way network latency
between these consecutive leaders. With the current stake-weighted random
leader schedule, consecutive leaders are independent of each other
geographically. This causes two problems:

- **Centralization.** Solana's validator stake is not uniformly distributed,
  but concentrated in certain areas. Under a random schedule, a validator
  located in a central location is on average close to its leader schedule
  predecessor and thus receives the previous block earlier. This rewards
  co-location and penalizes remote validators. The protocol should actively
  work against this centralization incentive.
- **Handover latency.** With Alpenglow's fast leader handover (SIMD-0630),
  the time between two leaders is dominated by the one-way latency from the
  previous leader to the next leader. A random schedule frequently puts
  intercontinental hops between leaders. As noted in SIMD-0630, this
  handover latency is also the main remaining reason why the wall-clock
  duration of an epoch exceeds its nominal duration.

This proposal clusters geographically close leaders into short runs. Within
a run, handovers are local and fast.

## Dependencies

This proposal depends on the following previous proposals:

- **[SIMD-0180]: Vote Account Address Keyed Leader Schedule** — Defines the
  stake-weighted random leader schedule that this proposal uses as its base
  schedule.
- **[SIMD-0674]: Validator Location Registration** — Defines how validators
  self-report their location and the distance measure `dist2` used in this
  proposal.

This proposal also changes a constant introduced in:

- **[SIMD-0630]: FLH Slot Time Compensation** — `HANDOVER_COMPENSATION` is
  tuned to the mean handover latency, which this proposal reduces.

Related proposals, not dependencies:

- **[SIMD-0326]: Alpenglow**
- **[SIMD-0337]: Markers for Alpenglow Fast Leader Handover** — Fast leader
  handover is what makes the leader-to-leader latency the dominant cost of a
  handover.

[SIMD-0180]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0180-vote-account-leader-schedule.md
[SIMD-0674]: https://github.com/solana-foundation/solana-improvement-documents/pull/674
[SIMD-0630]: https://github.com/solana-foundation/solana-improvement-documents/pull/630
[SIMD-0326]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0326-alpenglow.md
[SIMD-0337]: https://github.com/solana-foundation/solana-improvement-documents/pull/337

## New Terminology

- **Base schedule**: the stake-weighted random leader schedule of an epoch,
  as computed today (SIMD-0180), with one entry, the leader, per leader
  window.
- **Radius**: Each validator `v` has a radius `r(v)`, the smallest distance
  such that the amount of stake with distance at most `r(v)` (including `v`
  itself) is at least `STAKE_FLOOR` of the total stake. Validators in dense
  areas have a small radius, validators in sparse areas have a large radius.
- **Stake floor**: `STAKE_FLOOR` is a security constant, given as a fraction
  of the total stake, that sets how large each validator's radius is and
  thereby the minimum validator diversity of each bin.
- **Bin**: a temporary group of leaders collected during schedule
  generation. Each bin has an anchor, which is the leader that started the
  bin.
- **Run**: the sequence of leaders of a bin once they are added to the final
  schedule.

## Detailed Design

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and
"OPTIONAL" in this document are to be interpreted as described in
[RFC 2119](https://www.ietf.org/rfc/rfc2119.txt) and
[RFC 8174](https://www.ietf.org/rfc/rfc8174.txt).

### Constants

```
STAKE_FLOOR = 10%
RUN_LENGTH = 3
```

`STAKE_FLOOR` determines how close validators must be to be eligible to be
in a bin. Decreasing this parameter makes bins geographically tighter on
average and thus reduces average handover delay. However, tighter bins also
make it easier for a low-stake adversary to control an entire run at a time
and thus have disproportionately long runs on average.

`RUN_LENGTH` determines the size of bins, and as such the length of runs.
Increasing this parameter decreases the number of long hops per epoch and
thus reduces the average handover delay. However, larger runs also lead to
longer periods of time spent in a single jurisdiction, more correlated
failures, more frequent and longer runs of leaders controlled by a potential
adversary.

### Definitions

For any two validators `u, v`, let `dist2(u, v)` be the squared chord
distance between the locations of `u` and `v`, in square meters, as defined
in SIMD-0674. Let `V` be the set of active validators (with non-zero stake
and a valid reported location) and let `S` be their total stake. For `v` in
`V`, the radius `r(v)` is the smallest `DIST` such that

```
sum(stake(u) for u in V if dist2(v, u) <= DIST) >= STAKE_FLOOR * S
```

The sum includes `v` and all other validators with exactly the same location
as `v`, i.e., `r(v) = 0` if the location has at least `STAKE_FLOOR` stake.

The stake and the locations used MUST be taken from the same snapshot that
is used for computing the base schedule of the epoch, so that all validators
agree on the result.

### Geo-Aware Leader Schedule

The geo-aware leader schedule for an epoch is computed in two steps. First,
the base schedule `L = [l_0, ..., l_{N-1}]` is computed exactly as today
(SIMD-0180), where each `l_i` is the `i`-th leader of the epoch in this base
schedule. Second, `L` is reordered, producing the final schedule `out`.

All randomness in this step MUST come from a deterministic PRNG seeded from
the epoch, domain-separated from and independent of the PRNG stream used for
the base schedule.

```
bins = []   // ordered by creation time
out = []

for l in L:
    near = [b for b in bins if dist2(l, b.anchor) <= max(r(b.anchor), r(l))]
    if near is empty:
        b = Bin { anchor: l, members: [l] }
        bins.append(b)
    else:
        b = near[uniform random index in 0..len(near)]
        b.members.append(l)
    if len(b.members) == RUN_LENGTH:
        out.extend(shuffle(b.members))
        bins.remove(b)

// finally, flush the remaining partially filled bins
for b in shuffle(bins):
    out.extend(shuffle(b.members))
```

`shuffle` MUST be a Fisher-Yates shuffle driven by the PRNG above.

In words: leaders of the base schedule are considered one by one, in order.
Leader `l` is near for bin `b` if `l` is within the radius of `b.anchor`, or
the anchor of `b` is within the radius of `l`. If no bin is near, `l` opens
a new bin; otherwise, `l` joins a near bin chosen uniformly at random. As
soon as a bin holds `RUN_LENGTH` leaders, it is appended to the schedule in
uniformly random order, and the bin is removed. At the end of the epoch, all
remaining bins are appended in uniformly random bin order, each in uniformly
random member order.

### Choice of Parameters

`STAKE_FLOOR` is a security parameter: higher values increase security but
reduce latency gains. `STAKE_FLOOR` SHOULD stay below the stake of the
smallest continent that should be able to fill bins on its own; at the time
of writing this is the case for `STAKE_FLOOR = 10%`.

`RUN_LENGTH` trades off latency gains against the length of time spent in a
single region. Choosing this parameter is non-trivial, and we propose to
have a community discussion. For the moment we work with `RUN_LENGTH = 3`,
as discussed in the Run Length Simulation below.

SIMD-0630 defines `HANDOVER_COMPENSATION` to be 25 ms. When activating this
SIMD, `HANDOVER_COMPENSATION` MUST change to 50 ms. This is derived from
simulation results using real-world network latency data and mainnet stake
distribution and node locations as of epoch 1038. With the geo-aware leader
schedule, the simulation shows that the average delay between receiving the
block directly from the previous leader to issuing the ParentReady event
increases from 23.2 ms to 46.2 ms. The choice of this value SHOULD be
revisited in the future, especially as significant geographic shifts in the
stake distribution happen.

### Run Length Simulation

We simulated the schedule on the mainnet stake distribution of epoch 1038,
with 661 validators at locations corrected by Globalping measurements (round
trips and block timing rather than IP geolocation). Each epoch has 108,000
leader windows, and results are averaged over 5 random seeds, with
`STAKE_FLOOR = 10%`. Bins are formed on the geographic distance between
registered locations, as specified above. Latency is used only to evaluate
the result: each validator is mapped to the nearest RIPE Atlas metro, and
the one-way latency between two validators is half the median round-trip
time measured between RIPE Atlas anchors in their metros (0 within a metro).

The simulated adversary controls 5% of the total stake and is placed in
Sydney, far from all other stake: no other validator is in Oceania, and the
nearest stake, in Los Angeles, is 73 ms away. This is close to a worst-case
attacker: an isolated adversary opens bins that no honest validator is near,
and therefore fills entire runs alone. An adversary in a dense location is
much less of a threat, since its bins mostly fill with honest local stake.

For each value `R = RUN_LENGTH`, the table shows the mean handover latency
in milliseconds over handovers between honest validators, the median
handover latency over all handovers (all validators at their true
locations), and the distribution of byzantine runs, as the percentage of the
adversary's runs having length 1, 2, ..., 8 and above. `R = 1` is today's
(SIMD-0180) random schedule.

|  R | mean | median |    1 |    2 |    3 |    4 |    5 |    6 |    7 |  8+ |
|---:|-----:|-------:|-----:|-----:|-----:|-----:|-----:|-----:|-----:|----:|
|  1 | 36.2 |   23.4 | 94.7 |  5.0 |  0.3 |  0.0 |    0 |    0 |    0 |   0 |
|  2 | 22.3 |    5.7 | 55.2 | 44.0 |  0.4 |  0.3 |  0.0 |    0 |    0 |   0 |
|  3 | 17.0 |    4.5 | 53.5 | 24.0 | 22.3 |  0.1 |  0.0 |  0.0 |    0 |   0 |
|  4 | 14.3 |    4.0 | 51.3 | 24.1 | 12.8 | 11.7 |  0.0 |  0.0 |  0.0 | 0.0 |
|  5 | 12.6 |    3.5 | 49.0 | 23.9 | 12.8 |  7.5 |  6.8 |  0.0 |    0 | 0.0 |
|  6 | 11.5 |    3.1 | 47.0 | 24.1 | 13.4 |  7.2 |  4.1 |  4.2 |    0 |   0 |
|  7 | 10.8 |    2.4 | 46.8 | 22.9 | 12.8 |  8.1 |  4.4 |  2.6 |  2.4 |   0 |
|  8 | 10.2 |    0.0 | 44.9 | 23.9 | 13.6 |  7.7 |  4.3 |  2.7 |  1.4 | 1.6 |
|  9 |  9.7 |    0.0 | 45.2 | 23.0 | 13.3 |  8.2 |  4.4 |  2.4 |  1.9 | 1.6 |
| 10 |  9.4 |    0.0 | 44.0 | 23.7 | 13.1 |  7.7 |  5.0 |  2.6 |  1.7 | 2.1 |
| 11 |  9.1 |    0.0 | 42.9 | 23.6 | 13.7 |  8.1 |  4.8 |  2.8 |  1.9 | 2.2 |
| 12 |  8.8 |    0.0 | 42.1 | 23.6 | 13.6 |  8.6 |  4.9 |  2.8 |  2.1 | 2.4 |
| 13 |  8.6 |    0.0 | 42.4 | 23.1 | 13.7 |  8.5 |  4.7 |  3.0 |  1.9 | 2.6 |
| 14 |  8.4 |    0.0 | 42.2 | 23.0 | 13.6 |  8.5 |  4.7 |  3.2 |  1.9 | 2.9 |
| 15 |  8.3 |    0.0 | 41.1 | 23.7 | 13.2 |  8.7 |  5.1 |  3.3 |  2.0 | 2.9 |

`0` means no such run occurred; `0.0` means some did, fewer than 0.05%. From
`R = 8` on, the median is 0 because more than half of all handovers stay
within one metro, which the RIPE Atlas matrix prices at 0 ms.

A run in the final schedule can occasionally exceed `RUN_LENGTH`, because
the adversary can end one run and start the next one; `RUN_LENGTH` bounds
the run within a single bin. At `R = 3` the longest byzantine run observed
was 6 windows, two bins back to back.

Small values of `RUN_LENGTH` already capture most of the latency gain.
`R = 3` halves the mean handover latency (36.2 to 17.0 ms), reaching 74% of
the improvement of `R = 8`. Byzantine runs longer than 3 remain rarer (0.2%
of runs) than runs longer than 2 are today (0.35%), and a regional outage
affects at most 3 consecutive leader windows (2.4 seconds) within a bin.

`RUN_LENGTH = 3` is therefore the security-conservative choice, and
`RUN_LENGTH = 8` (the knee of the mean-latency curve) the latency-lean
alternative. Values above 8 are not worth their cost: going from 8 to 15
saves another 1.9 ms, against 26.0 ms from 1 to 8.

### Properties

The geo-aware leader schedule achieves the following properties:

- **Short handover latency.** By design, all but the first leader of each
  run (hence with at least probability `1 - 1/RUN_LENGTH`) have a
  geographically close predecessor. This reduces the average handover
  latency.
- **Decentralization.** A shorter handover latency reduces the advantage of
  being located at the gravity of the stake: Before, with the SIMD-0180
  random schedule, this probability was only `density`, where `density` was
  the local stake. Now it is `1 - 1/RUN_LENGTH` even for leaders in sparse
  areas.

Apart from these two properties mentioned in the Motivation section, our
protocol also achieves:

- **Proportional representation.** The number of leader windows of each
  validator is exactly the same as in the base schedule, since the algorithm
  only reorders `L`.
- **No long byzantine runs.** A run is deterministically bounded by
  `RUN_LENGTH`. To control a large share of a run, an adversary has to
  control a large share of the stake within the radius of its location, i.e.
  roughly `STAKE_FLOOR` of the total stake.
- **Natural schedule.** Since bins are flushed as soon as they are full, any
  medium-sized interval (say, 100 consecutive leaders) of the final schedule
  has approximately the stake distribution of the whole epoch. In
  particular, we do not have 100 consecutive leaders from the same
  geographical area.
- **Schedule symmetry.** Within a run, the order is uniformly random, so for
  two validators `A, B` in the same run, `A` directly preceding `B` is
  exactly as likely as `B` directly preceding `A`. This is also
  approximately true at the boundary of runs, because filling the bins is a
  stochastic process. A symmetric schedule has good properties, e.g.: If `A`
  would not send its block directly to `B` (SIMD-0630), `B` could retaliate
  when the roles are reversed. This is a repeated prisoner's dilemma which
  incentivizes good behavior.
- **Truthful location.** A validator reporting a false location is often
  preceded by leaders that are further away from its actual location, which
  increases its handover latency. The Location Fibbing Simulation below
  shows that misreporting the location does not (significantly) improve a
  validator's situation.

### Location Fibbing Simulation

To check whether a validator can gain by misreporting its location, we
simulated, for a set of cities, a validator that stays at its true location
but registers a different one. The liar is the largest validator in its
city, and every cell averages about 10,000 of its handovers over 800-window
epochs, priced on the true RIPE Atlas latencies. Each cell shows the mean
handover latency (in ms) experienced by that validator, at `RUN_LENGTH = 3`
and `STAKE_FLOOR = 10%`. Rows are the true location, columns the registered
location; the diagonal is truthful reporting. In this matrix the stake
radius and bin membership are measured in RIPE Atlas latency, not in
geographic distance.

|true \ reported|FRA|AMS|LON|IAD|LAX|TYO|SIN|HKG|SAO|SEL|
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|
|Frankfurt|**10.0**|12.4|11.8|33.6|34.4|55.1|55.1|55.1|27.3|53.0|
|Amsterdam|11.8|**10.5**|10.5|34.9|36.2|61.2|61.2|61.2|29.1|58.5|
|London|14.5|12.9|**11.3**|34.6|36.6|63.6|63.6|63.6|29.4|62.2|
|Ashburn|43.0|42.7|40.6|**23.7**|25.7|64.9|64.9|64.9|21.0|63.2|
|Los Angeles|69.4|67.6|66.7|40.5|**35.3**|64.1|64.1|64.1|41.9|63.7|
|Tokyo|109.4|108.5|106.5|82.9|75.1|**47.2**|47.2|47.2|84.1|47.8|
|Singapore|79.9|81.3|78.9|95.4|87.8|43.5|**43.5**|43.5|94.4|43.2|
|Hong Kong|92.3|93.0|91.8|94.4|88.1|47.5|47.5|**47.5**|97.4|47.2|
|São Paulo|100.1|99.8|98.3|78.3|83.1|133.7|133.7|133.7|**74.7**|133.8|
|Seoul|139.0|136.1|135.9|109.0|101.8|74.6|74.6|74.6|111.6|**73.8**|

In nine of the ten rows the diagonal is the minimum, up to latency-
equivalent registrations such as Tokyo, Singapore and Hong Kong; no other
lie gains more than 0.3 ms. The exception is Ashburn, which gains
2.7 +/- 0.2 ms by registering São Paulo. Truthfully, it sits just inside the
31 ms radius of Los Angeles and sometimes joins west-coast bins. Registered
in São Paulo, it falls outside that radius but stays within São Paulo's own
65 ms radius, which covers the US east coast, so it joins nearby east-coast
bins instead.

### Edge Cases

- **`RUN_LENGTH = 1`.** Every bin is flushed immediately, and the final
  schedule equals the base schedule.
- **Validator above `STAKE_FLOOR`.** Such a high stake validator `v` has
  radius `r(v) = 0`. There is a chance for `v` to have a monopoly run.
- **Equal distances.** Being near uses `<=`, and `r(v)` is defined via
  `>=`, so ties are resolved deterministically.

## Alternatives Considered

- **Different base schedules.** Instead of today's stake-weighted random
  sampling, the base schedule could be computed for the whole epoch at once,
  with each validator at most one leader window off its expected number
  (e.g. via pipage rounding), or even with deterministic short-term fairness
  across epochs (systematic sampling). These reduce the variance of the
  number of leader windows per validator. These improvements are orthogonal
  to this proposal and can be introduced later.
- **Static geographic regions.** Clustering leaders by fixed regions (e.g.
  continents) is simpler, but small stake regions such as Latin America and
  Africa would let a small adversary control long runs. Defining regions
  manually would need to be revisited as the stake distribution changes,
  while the radius computation adapts automatically.
- **Globally latency-optimal ordering.** Ordering the schedule to minimize
  the total handover latency (e.g. as a traveling-salesperson tour) would
  schedule entire regions contiguously. This violates the natural schedule
  property and leads to very long runs within a single jurisdiction.
- **Frame-based solutions.** Start with the SIMD-0180 random leader
  schedule. Consider a frame of `RUN_LENGTH` consecutive leaders, and just
  reorder them in place. This is a reasonable solution, but it incentivizes
  centralization. A leader in a sparse area (with stake less than
  `1/RUN_LENGTH`) only rarely has a predecessor in the same area, while a
  leader in a dense area often has a nearby predecessor. So there is an
  incentive to move to a dense area, which means centralization.
- **Do nothing.** Keeps the current incentive to co-locate with the majority
  of the stake and the current handover latency. It is the most secure
  against byzantine runs.
- **Geographic order within a run.** Instead of a uniformly random order
  within a run, the first leader is chosen at random and the remaining
  leaders follow a shortest geographic path through the bin. Only the first
  leader of a run pays a long handover; all further handovers become even
  shorter, which would further improve the average latency. However, given
  the bin membership, the order within a run is then largely deterministic:
  the schedule symmetry property is weakened, and nearby validators are
  always scheduled adjacently, so an adversary with several co-located
  validators gets predictable, consecutive leader slots.

## Impact

- **Users and dapp developers.** Lower average handover latency means less
  idle time between leaders, and thus more slots per unit of wall-clock
  time. Transaction senders SHOULD NOT assume that consecutive leaders are
  geographically independent.
- **Validators.** Validators in sparse locations benefit most, as they are
  much more likely to have a nearby predecessor than today. The number of
  leader windows a validator gets in an epoch is unchanged, only their
  respective order changes.
- **Core contributors.** The leader schedule computation becomes more
  complex and depends on reported locations.
- **`HANDOVER_COMPENSATION`.** `HANDOVER_COMPENSATION` of SIMD-0630 is tuned
  to the mean handover latency and MUST be changed from 25 ms to 50 ms.

## Security Considerations

- **Self-reported locations are not verifiable.** A validator can misreport
  its location. As argued in the properties above, misreporting mostly harms
  the validator themselves.
- **Byzantine runs.** Runs are bounded by `RUN_LENGTH`, and an adversary
  needs roughly `STAKE_FLOOR` of the total stake concentrated in one area to
  control full runs. Decreasing `STAKE_FLOOR` or increasing `RUN_LENGTH`
  weakens this bound.
- **Correlated failures.** A regional outage (power, network, jurisdiction)
  now affects consecutive leaders, which leads to longer sequences of
  skipped slots compared to a fully random schedule. With `RUN_LENGTH = 3`,
  4 slots per leader window and 200 ms slots, a run ideally spans 2.4
  seconds.
- **Jurisdictional concentration.** For the duration of a run, block
  production happens within a single region, which could make regional
  censorship somewhat more effective in that time frame.

## Drawbacks

- The leader schedule computation is more complex and depends on an
  additional input (reported locations) that has to be agreed upon.
- The benefit depends on validators reporting their locations truthfully and
  on the accuracy of the distance measure as a proxy for network latency.
- Both parameters are tuned constants that trade off latency against
  security and may need to be revisited as the stake distribution changes.

## Backwards Compatibility

The leader schedule is consensus-critical, so a feature gate is REQUIRED. If
the feature gate is activated in epoch `e`, the new algorithm MUST be used
to calculate the leader schedule for all epochs starting with `e+2`, and the
old SIMD-0180 algorithm MUST be used for all epochs up to and including
`e+1`. The format of the leader schedule, e.g. as reported by RPC, is
unchanged. Ledger replay of historical epochs is unaffected.
