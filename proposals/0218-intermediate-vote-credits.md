---
simd: '0218'
title: Intermediate vote credits
authors:
  - Ashwin Sekar
category: Standard
type: Core
status: Review
created: 2025-01-04
feature: (fill in with feature tracking issues once accepted)
extends: SIMD-0033 Timely Vote Credits
---

## Summary

This feature aims to build upon the effort of timely vote credits in order
to improve the vote credit algorithm.

Credits are awarded for a block voted on by a validator that ultimately gets
finalized. For consensus purposes a vote for a block is implicitly a vote for
all its ancestor blocks as well. We propose a mechanism in which all intermediate
blocks between the previous root and the new root on a fork are awarded credits.

## Motivation

Although greatly improved via TVC, the credit accounting algorithm is still lacking
in measuring the contribution of validators to consensus. There still exist
gaps whereby validators can gained increased credits through modifications that
do not have any consensus benefits. Running unaudited code poses a risk to the network.

Since there is no consensus benefits to these modifications, by aligning the credit
algorithm to more accurately reflect consensus contribution, we hope to reduce the
potential danger of these modifications.

Similarly, honest validators are not fully rewarded for all of their contributions.
This SIMD aims to take a step in reducing the gap and making the credit algorithm
more representative of consensus contributions.

## New Terminology

N/A

## Detailed Design

Validators earn credits based on the latency of their votes on blocks that
eventually get rooted. Occasionally validators will have gaps in voting due to
forking, safety checks or other factors outside of their control.

Consider this example where a validator missed out on voting for blocks 103 and
104 as it was momentarily tricked by a minor fork occuring. It otherwise submitted
latency 2 votes:

```
[100] - [101] - [103] - [104] - [105]
 v 2     v 2                     v 2
            \ - [102]
                 v
```

Assuming this top fork gets finalized, the validator will receive 3 * 16 = 48 credits.
Although the validator did not vote on 103 or 104, its timely vote on 105 conveyed
its vote on these intermediate blocks implicitly. Thus we propose that the
intermediate blocks should be considered as having been voted on in the block in
which the vote for 105 landed for the purpose of accounting:

```
[100] - [101] - [103] - [104] - [105]
 v 2     v 2     i 4     i 3     v 2
            \ - [102]
                 v
```

With this accounting, the validator earns 77 credits.
As the vote for 105 landed in 107, we treat any intermediate block as also having
a vote landed in 107. In fact this is the approach many "backfill" mods take,
by stuffing all missed slots in the first vote after a gap.

This backfilling is an unecessary transmission of information which also increases
lockout with no basis. Instead we modify the credit algorithm to recognize the vote
after a gap as a vote for the intermediate ancestors as well.

Note that this does not remove the competitive advantage of being a high performing
validator. If for example the same validator did not vote on the minor fork:

```
[100] - [101] - [103] - [104] - [105]
 v 2     v 2     v 2     v 2     v 2
            \ - [102]

```

It would earn 80 credits.

More formally when the feature flag `enable_ivc: EivcMRDPBKcmkgfWZ7p317A6JPshcDBStgAGSogerDpq`
is activated the vote program must:

* Collect all ancestors `A` between the previous root and the new root as reported
  via the `SlotHashes` sysvar when rooting a lockout `r` with latency `l`
* Assign each ancestor `a` in `A` latency of `l + (r - a)`
* Then award credits for each slot in `A` and `r` using the scheme in SIMD-0033

## Alternatives Considered

N/A

## Impact

This proposal will more fairly assign credits to consensus contributions and remove
the unfair advantage of modifications that do not contribute any consensus value.

## Security Considerations

N/A

## Backwards Compatibility

The feature is not backwards compatible
