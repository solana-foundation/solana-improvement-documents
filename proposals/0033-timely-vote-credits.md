---
simd: '0033'
title: Timely Vote Credits
authors:
  - Bryan Ischo <bryan@ischo.com>
category: Standard
type: Core
status: Implemented
created: 2023-01-30
feature: [tvcF6b1TRz353zKuhBjinZkKzjmihXmBAHJdjNYw1sQ](https://github.com/solana-labs/solana/issues/32857)
development: 
 - Anza - implemented
 - Firedancer - implemented
---

## Summary

Award a variable number of vote credits per voted on slot, with more credits
being given for votes that have "less latency" than votes that have "more
latency".


## Motivation

Vote credits are the accounting method used to determine what percentage of
inflation rewards a validator earns on behalf of its stakers.  Currently, when
a slot that a validator has previously voted on is "rooted", it earns 1 vote
credit.  A "rooted" slot is one which has received full committment by the
validator (i.e. has been finalized).

One problem with this simple accounting method is that it awards one credit
per rooted slot regardless of when it was voted on.  This means that a
validator can delay its voting for many slots in order to survey forks and
make votes that are more likely to be rooted.  This strategy saves fees on
extraneous votes, and minimizes no voting periods due to lockout.

This is not just a theoretical concern; a number of validators appear to be
using this technique to inflate their vote credits, some having been doing so
for over a year.  The number of validators using this approach is increasing
over time.  It is estimated that about 7.5 million stake, or roughly 2% of all
stake on the network, is using an intentional vote lagging strategy.

## Alternatives Considered

It's hard to conceive an alternative to this proposal since the problem can
only be addressed via economic incentive to reduce vote lagging.  The only
alternatives would be in the specific characteristics of how credits are
awarded based on vote latency; several parameterizations were considered and
this was written about in an early description of this issue:

https://www.shinobi-systems.com/timely_voting_proposal

This website is quite out of date however; the parameterizations that were
proposed there ("4 16 1", see later in this document for a description of what
these numbers mean) has been long since superceded by different values ("2 8
1").  This is because the overall performance of the Solana mainnet cluster
has matured to a degree where "tighter timings" are tolerable.

## New Terminology

**Vote Latency** is defined as the difference, in slots, between the slot
being voted on within a vote transaction, and the slot in which the vote
transaction itself lands.  The minimum possible vote latency for any voted-on
slot is 1, since a vote on a slot must land no earler than the slot
immediately following the slot being voted on.

For example, if a vote transaction which included slot 1000 were voted on in a
transaction that was included in the block at slot 1003, then the vote latency
for the vote on that slot would be 3.

## Detailed Design

Credits awarded are a function of this latency, with lower latencies awarding
more credits.  This discourages intentional "lagging", because delaying a vote
for any number of slots decreases the number of credits that vote will earn,
because it will necessarily land in a later slot if it is delayed, and then
earn a lower number of credits than it would have earned had it been
transmitted immediately and landed in an earlier slot.

If landing a vote with 1 slot latency awarded more credit than landing that
same vote in 2 slots latency, then validators who could land votes
consistently within 1 slot would have a credits earning advantage over those
who could not.  Part of the latency when transmitting votes is unavoidable as
it's a function of geographical distance between the sender and receiver of
the vote.  The Solana network is spread around the world but it is not evenly
distributed over the whole planet; there are some locations which are, on
average, more distant from the network than others are.

It would likely be harmful to the network to encourage tight geographical
concentration - if, for example, the only way to achieve 1 slot latency was to
be within a specific country, then a very strict credit rewards schedule would
encourage all validators to move to the same country in order to maximize
their credit earnings.

For this reason, the credits reward schedule has a built-in "grace period"
that gives all validators a "reasonable" amount of time to land their votes.
This reduces the credits earning disadvantage that comes from being more
distant from the network.  A balance has been struck between the strictest
rewards schedule, which most strongly discourages intentional lagging, and
more lenient rewards schedules, which improves credit earnings for distant
validators who are not artificially lagging.

Historical voting data has been analyzed over many epochs and the data shows
that the smallest grace period that allows for very minimal impact on well
behaved distant validators is 2 slots, which means that all slots voted on
within 2 slots will award maximum vote credits to the voting validator.  This
gives validators about 1 second to land their votes without penalty.  The
maximum latency between two points on Earth is about 100 ms, so allowing a
full 1,000 ms latency without penalty does not have adverse impact on distant
validators.

Another factor to consider is what the maximum vote credits to award for a
vote should be.  Assuming linear reduction in vote credits awarded (where 1
slot of additional lag reduces earned vote credits by 1), the maximum vote
credits value determines how much "penalty" there is for each additional slot
of latency.  For example, a value of 10 would mean that after the grace period
slots, every additional slot of latency would result in a 10% reduction in
vote credits earned as each subsequent slot earns 1 credit less out of a
maximum possible 10 credits.

Again, historical voting data was analyzed over many epochs and the conclusion
drawn was that a maximum credits of 8 is the largest value that can be used
and still have a noticeable effect on known laggers.  Values higher than that
result in such a small penalty for each slot of lagging that intentional
lagging is still too profitable.  Lower values are even more punishing to
intentional lagging; but an attempt has been made to conservatively choose the
highest value that produces noticeable results.

When a Vote or VoteStateUpdate instruction is received by a validator, it uses
the Clock sysvar to identify the slot in which that instruction has landed.
For any newly voted on slot within that Vote or VoteStateUpdate transaction,
the validator will record the vote latency of that slot as (voted_in_slot -
voted_on_slot).

These vote latencies are stored per vote in a u8 as part of the on chain vote
state.

The credits awarded for a rooted slot are calculated using the latency value
for the corresponding vote, using a formula that awards latencies of 1 - 2
slots 8 credits, with a 1 credit reduction for each vote latency after 2.
Rooted slots will always be awarded a minimum credit of 1 (never 0) so that
very old votes, possibly necessary in times of network stress, are not
discouraged.

The implementation of the above is fairly simple for Vote transactions,
because Vote transactions include slot numbers voted on, and that list of
slots is already pruned of already-voted-on slots; so each newly voted-on slot
that survives this pruning can have its latency calculated simply.


## Impact

1. Any code which deserialized vote account state will require a recompile
   with the newest Solana SDK in order to be able to deserialize the new vote
   account state.

2. There will be a strong anti-incentive for vote lagging.  Validators who
   currently delay their votes in order to reduce their chances of being
   locked out from voting, will no longer do so.

3. The Solana CLI will have been updated to show vote latencies for each vote;
   any context in which the Solana CLI output is observed by users or other
   tooling will need to adapt to the additonal data being displayed.

4. The maximum number of earnable credits will increase from 432,000 to
   3,456,000.  This will add an extra digit in any context where this value is
   displayed, which could make the tabular form of some tooling too large to
   fit the predefined columns.

5. The format of data stored in vote accounts will change; see the Backwards
   Compatibility section for details.

## Security Considerations

The are no security considerations; however there are changes to incentive
structure for voting which may result in differing vote behavior becoming more
optimal with regards to earning credits.

To maximize vote credits, a validator must vote on all slots that end up being
rooted by the cluster, and no slots that are not rooted.  This means always
"picking the right fork" whenever there are forks.  It is the drive to always
vote on the correct fork that causes validators to choose a vote lagging
strategy in the first place; they can earn more credits if they reduce their
chances of voting on "the wrong fork" by waiting until sufficient votes from
other validators have landed on a given slot before voting on it.

Therefore the optimal vote strategy currently is to wait until a slot is
rooted (i.e. has received 31 confirmations) before voting on it.  In this way,
perfect vote credits would be earned.  However, this optimal voting strategy,
if employed cluster-wide, would halt the cluster as no validators would vote,
all of them waiting for others to vote first.

It's possible to get very close to this optimal strategy by waiting for some
number of confirmations less than 31, with the number of incidents of picking
the wrong fork increasing as the waiting time is decreased.  The current "vote
laggers" have done this, with varying degrees of skill.

After the timely vote credits change, the dynamics will change; it will no
longer be profitable to intentionally lag, or if it is profitable in some
situations to do so, the frequency and duration of this situations will be
vastly reduced.

The new optimal voting strategy will be a complex calculation based partially
on predictions of how frequently forks will occur and how long they will last.
This is because a validator who votes more quickly will earn more vote credits
per vote, but will also be more likely to have voted on a dead fork and end up
being locked out for some number of slots.

It will become impossible to pick a voting strategy that always produces the
most possible vote credits.  Different voting strategies will all exist
simultaneously, in a sort of a "rock paper scissors" situation where the best
possible strategy shifts over time depending on how other validators are
voting.  This will result in the strengthening of the cluster via the
elimination of a "voting monoculture".


## Backwards Compatibility

Because a new field is added to the Lockout struct, the format of the
VoteState struct, which is Rust bincode serialized and stored in the Vote
account for each validator, is changed.  This requires adding a new enum value
to VoteStateVersions, which then makes any code outside of the validator code
base which does not know about this new VoteStateVersion unable to deserialize
vote account contents.  This will require some announcement and for
applications which deserialize Vote accounts to be given time to recompile
with the new Solana SDK that includes the new VoteStateVersion.

In addition, when VoteState updates, any validators running a version of the
code from before the addition of the new VoteStateVersion will be unable to
deserialize vote accounts loaded from snapshots produced by validators running
newer versions of the code.  This could create difficulties for validator
operators who download snapshots and find that their validator cannot process
vote transactions due to the vote accounts being unloadable.

For this reason, the change must be broken into several parts, with enough
time between changes to allow the validator set to all move well beyond
incompatible code versions:

1. A change will be made which adds the new VoteStateVersion to the code base
   but does not enable its use.  The enablement of the use of the new
   VoteStateVersion will be behind a feature switch; until this feature is
   enabled, validators will internally represent Vote accounts using the new
   VoteStateVersion, but will always serialize and store vote account contents
   using the prior version.  This does not lose information as the latency
   value will not be used for anything before a later change, and thus is
   implicitly always valued as '0', and on reading the vote account and
   converting it to the newest VoteStateVersion, the default value of '0' for
   latency will reconstitute the value of 0 that was in that field at the time
   that the VoteState was serialized.

   Once this software version has been integrated into the cluster and proven
   for a period of time, would the feature be enabled.  At this point,
   validators will begin writing vote account state using the newest
   VoteStateVersion, and their snapshots will no longer be usable by the old
   software versions, which by this point will be so old that rollback to them
   is a practical impossibility anyway.

   In addition, the vote state of an account cannot be updated to the new
   larger form unless the vote account has sufficient lamports to cover the
   new higher rent exempt minimum.  The new size of vote accounts will be 3762
   bytes, a 31 byte increase on the old size of 3731 bytes.  The rent exempt
   minimum for vote accounts will thus increase from 0.02685864 SOL to
   0.0270744 SOL.  This is an additional 0.00021576 SOL per vote account.
   Given roughly 2,000 vote accounts currently on mainline, the maximum SOL
   needed to increase all vote accounts to the required new minimum is 0.43152
   SOL.  The true value will certainly be less than this because some vote
   accounts will already be above the new minimum.  In any case, the author of
   the SIMD will send any additional SOL needed to increase rent exempt
   minimum of all vote accounts, in the weeks leading up to the enablement of
   this feature.  By the time the feature is enabled, the vast majority of
   vote accounts will be funded to the new rent exempt minimum.

   For any vote account not sufficiently funded, the old VoteState version
   will be written.  Since the old VoteState does not record latencies, vote
   account deserialization will assign the default value of 0 to the Lockout
   latency field, which will result in the minimum vote credits awarded for
   each vote for that vote account.  This provides economic incentive to
   validators to sufficiently fund their vote accounts to allow the updated
   VoteStateVersion to be serialized into them.

   To summarize the issues surrounding the Vote account updates:

   - A vote account will only be updated to the new version after the feature
     is enabled, and only if the vote account includes enough SOL to satisfy
     the new rent exempt minimum.
     
   - If a vote account is not updated, then it will continue to work, but
     because its vote latencies will not be recorded, and will default to 0,
     the vote account will earn minimum credits for votes until updated

   There is no danger of breaking vote processing; the only danger is to
   validators who do not supply sufficient rent exempt minimum, and that
   danger is limited to reduced earnings through lower vote credits.  There is
   no risk to cluster consensus.

2. A second change will be made, with a separate feature, that will enable the
   recording of the latency values for votes in the latency field of Lockout,
   and the subsequent use of this latency value to compute vote credits
   awarded.

   Once this feature is enabled, vote accounts will earn credits based on
   their vote latencies.  The maximum possible credits earnings per epoch will
   increase from 432,000 to 3,456,000.  Each validator will continue to earn
   vote rewards based on their fraction of total vote credits earned across
   the cluster, and validators which earn more credits through reduced vote
   latencies will earn a larger percentage of the total rewards.
