---
simd: '0009'
title: Lockout Violation Detection
authors:
  - carllin
  - ashwinsekar
  - wencoding
category: Standard
type: Core
status: Accepted
created: 2022-12-12
feature: (fill in with feature tracking issues once accepted)
---

## Summary

An algorithm designed to catch validators that violate lockout rules when
voting.

## Motivation

Validators that violate lockout rules unfairly earn rewards and put cluster.
consensus at risk

## Alternatives Considered

None

## New Terminology

None

## Detailed Design

Assume:

1. A database that we will store vote information ingested from
gossip/turbine, which in turn will be consulted to detect lockout violations
as follows.

2. Tracking of all SlotHashes on the canonical/rooted fork in the database
for the last epoch.

The following sections will go over the possible lockout violations that will
be detected and punished.

### Rooting a different fork

If there are any votes made that have a root `R` missing from the rooted
SlotHashes list, slash them on the rooted fork since they have committed
to a different fork

### Removing lockouts

Attempting to illegally *remove* a lockout in a newer vote that that should
have still existed based on an older vote is a lockout violation. We detect
this as follows:

1. For all non root slots `S` in each vote, track a range `(S, S + 2^n)` where
`n` is the confirmation count. For each slot `S` we only have to track the
greatest such lockout range in the database.

2. For each new vote `V`, for each slot `S` in the vote, lookup in the database
to see if there's some range `(X, X + 2^n)` where `S` is in this range, but
`X` is missing from the vote `V`. This is a lockout violation because this
implies that the validator made a vote where `S` popped off `X`, but the
lockout on `X` from an earlier vote should have prevented that from happening.

Note for each interval `(S, S + 2^n)` we also need to be able to lookup the
vote. This is important for being able to pull up the votes later as part of a
proof.

### Reducing lockout

Attempting to illegally *reduce* a lockout from an older vote in a newer vote
is a lockout violation.

The tricky bit of this is determining that one vote `V` is *older* than
another vote `V'`. To this end, we track a flag `is_vote_earlier` whose usage
we will highlight in the protocol below.

1. We set `is_vote_earlier = false`.

2. For each newly seen vote `V` made by a validator `X`, for each slot `S` in
`V`:
    - If `S` exists in the database:
        - Compare the lockout `L_V` in the vote `V` on `S` against the greatest
        lockout `L_D` in the database for that slot `S` made by that validator
        `X`.
            - If `L_V` < `L_D`, set `is_vote_earlier=true`.
            - If `L_V` == `L_D`, continue.
            - If `L_V > L_D`, check if the flag `is_vote_earlier=true`. If so,
            this implies this vote `V` was *older* than some vote `V'` that
            comitted the greater `S'` lockout to the database, yet `V` has a
            lesser lockout on `S`, which means the validator reduced lockout on
            `S` in a later vote. This is a lockout violation.
    - If `S` does not exist in the database, the above `Removing lockouts`
    section describes the protocol that will catch violations.

### Reducing roots

Reducing the root from earlier to later votes is a lockout violation. We detect
this as follows:

1. For each validator we track a rooted set in the database. We can remove an
interval `(S, S + 2^n)` from the database once the slot becomes a root add it
to a rooted set for this validator, and any new votes < root also get added to
rooted set.

2. When we see a vote with root N on the main fork, then we remove all
intervals `(M, P)` where `M < N && P >= N` and  add `M` to the rooted set.

So for example if we see:
(Assume `{slot: confirmation count}` format)

- `{root: 0, 1: 4, 3: 3, 5: 2, 7: 1}`

- `{root: 5, 7: 1}`

- Then we add `{1, 3}` to rooted set and remove their intervals from the
interval tree because both of those are `< 5`, but have lockouts that extend
past `5`

Note here also that that artificially increasing your lockout is not a
slashable offense (here we root 5 however 7 still has a conf count of 1),
because adopting stricter lockout does not weaken commitment on any previously
committed fork.

Thus, if we then later saw a vote:

- `{1: 2, 2: 1}` on a different fork we would say it's slashable because the
lockout on 2 extended past a rooted slot 3 in the rooted fork, so 2 should have
prevented the vote for 3 from being made, evicted
- `{1: 2, 4: 1}` on a different fork, then because 3 was rooted and 3 does not
exist in this vote, it's implied 3 was popped off. However, 3 was rooted so it
couldn't have been popped off by 4, so that's slashable

Ordering here is tricky though, for instance what if we saw vote 2 then vote 1?
We would retroactively have to add `{1,3}` to the rooted set

Also note here that evicted slots will not be added to the rooted set. For
example, imagine:

- `{root: 0, 1: 3, 3: 2, 4: 1}`

- `{root: 0, 1: 4, 3: 3, 7: 2, 9: 1}`

- `{root: 7, 9: 2, 10: 1}`

Here we add `{1, 3}` to the rooted set, but 4 doesn't get added because
`4 + 2^1 < 7`, so it does not overlap the root of `7`. This means the interval
`(4, 4 + 2^1)` remains in the database. This is important because:

- If we see a vote `{root: 0, 1: 3, 3: 2, 5: 1}` on another fork, this is only
known to be slashable by seeing this interval `(4, 4 + 2^1)` (because it
doesn't include `4` in the vote, but `4's` lockout should have prevented it
from being popped off)
- We don't want to add `4` to the rooted set to prevent slashing a valid vote
on a different fork like `{root: 0, 1, 3, 10}`. If `4` was present in the
rooted set, we would report an error because `10` should not have popped off `4`

## Impact

Validators snitching on voting misbehavior will be more effective.

## Security Considerations

None

## Backwards Compatibility

Not applicable.
