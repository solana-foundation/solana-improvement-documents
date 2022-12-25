### Lockout Violation Detection

We assume a database that we will store vote information ingested from
gossip/turbine, which in turn will be consulted to detect lockout violations
as follows.

1) Track all SlotHashes on the main fork in the database.
2) If there are any votes made that have a root `R` missing from the SlotHashes
list, slash them on this fork
3) For all other slots `S` in each vote other than the root, track a range
`(S, S + 2^n)` where `n` is the confirmation count. For each slot `S` we only
have to track the greatest such lockout range in the database.
4) For each new vote `V`, for each slot `S` in the vote, lookup in the database
to see if there's some range `(X, X + 2^n)` where `S` is in this range, but
`X < S` and `X` is missing from the vote `V`. This is a lockout violation
because this implies that the validator made a vote where `S` popped off `X`,
but the lockout on `X` from an earlier vote should have prevented that from
happening.

Note for each interval `(S, S + 2^n)` we also need to be able to lookup the
vote. This is important for being able to pull up the votes later as part of a
proof.

5) For each validator we also track a rooted set in the database as well to
catch violating lockouts on rooted slots. We can remove an interval
`(S, S + 2^n)` from the database once the slot becomes a root add it to a
rooted set for this validator, and any new votes < root also get added to
rooted set.

When we see a vote with root N on the main fork, then we remove all intervals
`(M, P)` where `M < N && P >= N` and  add `M` to the rooted set.

So for example if we see:
(Assume `{slot: confirmation count}` format)
1. `{root: 0, 1: 4, 3: 3, 5: 2, 7: 1}`
2. `{root: 5, 7: 1}`
3. Then we add `{1, 3`} to rooted set and remove their intervals from the
interval tree because both of those are `< 5`, but have lockouts that extend
past `5`

Thus, if we then later saw a vote:

1. `{1: 2, 2: 1}` on a different fork we would say it's slashable because the lockout on 2 extended past a rooted slot 3 in the rooted fork, so 2 should have prevented the vote for 3 from being made, evicted
2. `{1: 2, 4: 1}` on a different fork, then because 3 was rooted and 3 does not exist in this vote, it's implied 3 was popped off. However, 3 was rooted so  it couldn't have been popped off by 4, so that's slashable

Ordering here is tricky though, for instance what if we saw vote 2 then vote 1? We would retroactively have to add `{1,3}` to the rooted set

Also note here that evicted slots will not be added to the rooted set. For example, imagine:
1. `{root: 0, 1: 3, 3: 2, 4: 1}`
2. `{root: 0, 1: 4, 3: 3, 7: 2, 9: 1}`
3. `{root: 7, 9: 2, 10: 1}`

Here we add `{1, 3}` to the rooted set, but 4 doesn't get added because `4 + 2^1 < 7`, so it does not overlap the root of `7`. This means the interval `(4, 4 + 2^1)` remains in the database. This is important because:

1. If we see a vote `{root: 0, 1: 3, 3: 2, 5: 1}` on another fork, this is only known to be slashable by seeing this interval `(4, 4 + 2^1)` (because it doesn't include 4 in the vote, but 4's lockout should have prevented it from being popped off)
2. We don't want to add `4` to the rooted set to prevent slashing a valid vote  on a different fork like `{root: 0, 1, 3, 10}`. If `4` was present in the rooted set, we would report an error because `10` should not have popped off `4`