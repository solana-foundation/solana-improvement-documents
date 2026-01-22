---
simd: '0363'
title: Simple Alpenglow Clock
authors:
  - Roger Wattenhofer
  - ksn6
category: Standard
type: Core
status: Review
created: 2025-09-19
---

## Summary

Since the clock sysvar computation is incompatible with Alpenglow, we need a new
design. In this document we suggest a simple replacement solution, which should
be accurate enough for all use cases.

## Motivation

So far, individual votes also include a timestamp, and the clock sysvar was
updated every slot by calculating the stake-weighted median of validator vote
provided times. With Alpenglow, individual votes do not go on chain anymore.
Consequently, we need a new clock design. 


## New Terminology

No new terms, but the following definition is updated: 

- Clock Variable in Solana Sysvar Cluster Data (unix_timestamp) is computed
  differently. The variable is still accessed by Clock::get() or by passing the
  sysvar account to a program as an account parameter. 


## Detailed Design

In Alpenglow, the current block leader includes an updated integer clock value
(Rust system time in nanoseconds) in its block *b* in slot *s* in the block
footer. This value is bounded by the clock value in the parent block.
Specifically, let the parent block be in slot *p* < *s*, and let the clock value
of slot *p* be *c* (in nanoseconds). For the clock value in slot *s* to be
correct, the clock value of block *b* must be strictly higher than *c*, and at
most *c*+(*s*–*p*) x 2*T*, where 2*T* is two times the block time, currently *T*
= 400 ms. We assume that a correct leader inserts its correct local time as long
as it is within the allowed bounds. If the correct local time is out of bounds,
the leader inserts the minimum or maximum allowed time.

If the clock value for slot s is not within the bounds (*c*, *c*+(*s*–*p*) x
2*T*], the proposed block *b* is invalid, and validators vote skip. Currently
2*T* = 8e8.

## Analysis

The design of this clock is simple on purpose to keep the overhead low. To the
best of our knowledge no program needs super accurate clocks, usually an
accuracy in the order of a few seconds is perfectly sufficient. 

The standard Alpenglow assumption is that we have less than 20% byzantine stake.
With at most 20% crashed stake in addition, we have at least 60% stake which is
correct. The 60% correct stake can correct any clock inaccuracies introduced by
the 20% byzantine stake. Slots are supposed to take 400 ms, and in reality slot
times are close to that value. 

For the analysis, let us assume that in each leader window, a leader is chosen
randomly according to our worst-case distribution, i.e., 20% of the leaders are
byzantine (assuming worst case behavior, i.e. corrupt leaders either always halt
the clock or push it maximally), 20% are skipped (the whole leader window is
only 400 ms instead of 1,600 ms), and 60% of the leaders are correct (fix the
clock as much as possible with the given bounds). 

We simulate the above algorithm with these worst-case assumptions. In the
simulation, the average clock skew we witness is about 1 second. For a 1 hour
window, the worst clock skew (the average largest skew in a 1 hour window) is
about 10 seconds. Such a high clock skew can happen if we are unlucky and we
experience several consecutive byzantine leader windows with the leaders either
halting the clocks or advancing them maximally.

In practice, we will probably see much lower levels of byzantine and crashed
leaders, which brings the average clock skew to around 20 ms.

## Implementation

We next articulate implementation details for the Alpenglow clock.

## Block Marker Modifications

In proposing a block, a leader will include a special marker called a "block footer," which stores a UNIX timestamp (in nanoseconds). As of the writing of this SIMD, the `block_producer_time_nanos` field of `BlockFooterV1` stores the clock:

```
/// Version 1 block footer containing production metadata.
///
/// The user agent bytes are capped at 255 bytes during serialization to prevent
/// unbounded growth while maintaining reasonable metadata storage.
///
/// # Serialization Format
/// ```text
/// ┌─────────────────────────────────────────┐
/// │ Producer Time Nanos          (8 bytes)  │
/// ├─────────────────────────────────────────┤
/// │ User Agent Length            (1 byte)   │
/// ├─────────────────────────────────────────┤
/// │ User Agent Bytes          (0-255 bytes) │
/// └─────────────────────────────────────────┘
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BlockFooterV1 {
    pub block_producer_time_nanos: u64,
    pub block_user_agent: Vec<u8>,
}
```

Upon receiving a block footer from a leader, non-leader validators will:

- Locally update the clock sysvar associated with the bank
- During replay, validate the bounds of this clock sysvar with respect to the parent bank's clock sysvar and proceed as outlined in "Detailed Design"

## Alpenglow Clock Variable Storage

The clock sysvar, as currently implemented, has second-resolution. This SIMD proposes a clock with greater resolution (nanoseconds). Accordingly, we introduce an off-curve account where we store the clock value upon processing the block footer (at the end of a block).

In particular, we calculate the off-curve account as follows:

```rust
/// The off-curve account where we store the Alpenglow clock. The clock sysvar has seconds
/// resolution while the Alpenglow clock has nanosecond resolution.
static NANOSECOND_CLOCK_ACCOUNT: LazyLock<Pubkey> = LazyLock::new(|| {
    let (pubkey, _) =
        Pubkey::find_program_address(&[b"alpenclock"], &agave_feature_set::alpenglow::id());
    pubkey
});
```

where `Pubkey::find_program_address` is a part of `solana-address-1.0.0`. In practice, the `Pubkey` ends up having value `BKRDmw2hTDSxQK4mitpK7eCWkNUvCvnaWqm1NZmGDTUm`.

The clock sysvar is updated at the end of each block by simply dividing the nanosecond timestamp by `1_000_000_000` and rounding down (i.e., integer division). For now, we do not expose the nanosecond clock to SPL programs via a sysvar; we may choose to do so in the future.


## Alternatives Considered

We discussed several designs for more complicated and potentially more accurate
clocks. In particular we considered clock constructions where nodes would only
consider a block correct if their internal clock (driven by the Internet Network
Time Protocol NTP) was “close enough” to the clock value proposed by the leader.
However, this design has many problems, and one can possibly even disadvantage
the next leader by pushing the clock to the allowed limit.

Alternatively, for a highly accurate clock, we need to have the nodes solve an approximate agreement
problem. This is similar to the current clock design, where all (or a selected
random sample of nodes) publish their local times, and then we (repeatedly) take
the median of the reported values. In principle, such an approach will give us a
very accurate clock. However, the cost of such a clock would be high. We believe
it is better if programs work with just reasonably accurate clocks.

## Impact

Developers and validators must not rely on a highly precise system clock. If an
application relies on a highly accurate clock, it should consider alternative
sources.

The semantics of the clock sysvar have slightly changed: The clock value does 
no longer represent the start time of the current block. It now represents the 
time at which the last slice of the parent block was produced.

Also, while the new clock has a nanosecond resolution, we compute a second 
resolution clock by dividing by 1_000_000_000 and rounding down.

## Security Considerations

A byzantine player could bribe a long series of consecutive leaders to either
halt the clock (or advance it 1s per slot) during a longer interval. If a
program accessing the clock is poorly designed and relies on a very accurate
clock, such an extended bribe could be profitable. If at the end of the bribe
period the clock was Delta (seconds) off, then it will take about Delta seconds
for the clock to go back to normal.

## Backwards Compatibility

This new clock is not as accurate as the current clock. So those users that run
programs that access the clock might need to adapt their code if they need a
high accuracy. We got the impression that this is not an issue in practice.

