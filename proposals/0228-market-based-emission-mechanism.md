---
simd: '0228'
title: Introducing a Programmatic, Market-Based Emission Mechanism Based on
Staking Participation Rate
authors:
  - Tushar Jain
  - Vishal Kankani
category: Standard
type: Core
status: Review
created: 2025-01-16
---


## Summary


SIMD-0228 Introduces a Market-Based emission mechanism based on staking
participation rates. This is the first of two SIMDs intended to make Solana
emissions more market oriented. This SIMD proposes a market-based mechanism to
dynamically determine Solana emissions.


## Motivation


As Solana matures, stakers increasingly earn SOL through mechanisms like MEV.
This income stream reduces the network's historical exclusive reliance on token
emissions to attract stake and security. According to Blockworks
(https://solana.blockworksresearch.com/), in Q4 2024 MEV, as measured by Jito
Tips, was approximately $430M (2.1M SOL),representing massive quarter-over-
quarter growth. In Q3 Jito Tips were approximately $86M (562k SOL), Q2 was
approximately $117M (747k SOL), and Q1 was approximately $42M (300k SOL).


Given the level of economic activity the network has achieved and the subsequent
revenue earned by stakers from MEV, now is a good time to revisit the network’s
emission mechanism and evolve it from a fixed-schedule mechanism to a
programmatic, market-driven mechanism.


The purpose of token emissions in Proof of Stake (PoS) networks is to attract
stakers and validators to secure the network. Therefore, the most efficient
amount of token issuance is the lowest rate possible necessary to secure the
network.


Solana’s current emission mechanism is a fixed, time-based formula that was
activated on epoch 150, a year after genesis on February 10, 2021. The mechanism
is not aware of network activity, nor does it incorporate that to determine the
emission rate. Simply put, it’s “dumb emissions.” Given Solana’s thriving
economic activity, it makes sense to evolve the network’s monetary policy with
“smart emissions.”


There are two major implications of Smart Emissions:


Smart Emissions dynamically incentivizes participation when stake drops to
secure the network.
Smart Emissions minimize SOL issuance  to the Minimum Necessary Amount (MNA) to
secure the network.


This is good for the Solana network and network stakers for four reasons:


High inflation can lead to more centralized ownership. To illustrate the point,
imagine a network with an exceedingly high inflation rate of 10,000%. People who
do not stake are diluted and lose ~99% of their network ownership every year to
stakers. The higher the inflation rate, the more network ownership is
concentrated in stakers’ hands after compounding for years.


Reducing inflation spurs SOL usage in DeFi, which is ultimately good for the
applications and stimulates new protocol development. Additionally, a high
staking rate can be viewed as unhealthy for new DeFi protocols, since it means
the implied hurdle rate is the inflation cost. Lowering the “risk free”
inflation rate creates stimulative conditions and allows new protocols to grow.


If Smart Emissions function as designed, they will systematically reduce selling
pressure as long as staking participation remains adequate. The inevitable side
effect and primary downside to high token inflation is increased selling
pressure. This is because some stakers in different jurisdictions have taken the
interpretation that staking creates ordinary income, and therefore they must
sell a portion of their staking rewards to pay taxes. This selling is a
significant detriment to the network and does not benefit the network in any
way.




In markets, sometimes perception is as important as reality. While SOL inflation
is technically not cost to the network, others think it is, and that belief
overall has a negative impact on the network. Inflation causes long-term,
continual downward price pressure that negatively distorts the market’s price
signal and hinders fair price comparison. To use an analogy from traditional
financial markets, PoS inflation is equivalent to a publicly listed company
doing a small share split every two days.




Historically, issuance curves have remained static due to Bitcoin’s immutability
ethos—a “Bitcoin Hangover” so to speak. While immutability suits Bitcoin’s
mission to become digital gold, it doesn’t map to Solana’s mission to
synchronize the world’s state at light speed.


In summary, the current Solana emissions schedule is suboptimal given the
current level of activity and fees on the network because it emits more SOL than
is necessary to secure the network. An issuance curve set by diktat is not the
right long-term approach for Solana. Markets are the best mechanism in the world
to determine prices, and therefore, they should be used to determine Solana’s
emissions.




## Detailed Design


### Five variables drive Solana’s staking market:
Yield for stakers (y)
Issuance Rate (i) - SOL emitted
SOL staked (N)
MEV in SOL terms (MEV)
Validator commissions (c)


These variables are mathematically related:




y = ((i+MEV)/N)*(1-c)


Currently, the network has a fixed issuance rate (i) while the number of SOL
staked (N) fluctuates based on market conditions. MEV also fluctuates based on
market conditions.


When considering new models for issuance, this relationship is critical.

Programmatic, Market-Based Emission Mechanism Based on Staking Participation
Rate


A dynamic, market-based rate can be determined using the following factors:


The Staking Participation Rate (s = SOL staked / Total SOL in existence) should
be based on what is needed for consensus safety.
The network should reduce issuance if the staking participation rate is higher
than the target rate and increase issuance if it is lower.
There should be a ceiling on the inflation rate as a protection mechanism.


We imagine the Target Staking Participation Rate (T) as a governable variable
and recommend a target staking participation rate of 50% for the following
reasons:
Beyond 67% incremental staked SOL does not add any incremental security
guarantees because a supermajority of all SOL has voted on any given block and a
long range attack is impossible. This “excess stake” explicitly inhibits network
economic activity and hampers growth.
Below 33%, we potentially risk network safety because a supermajority of all SOL
has explicitly not voted on any given block and this opens the edge case
possibility of long range attacks.


It also proposes the following bounds for the issuance rate:
Upper Bound: The current Solana issuance curve (decreasing at a rate of 15% per
year and will stop decreasing once nominal inflation is 1.5%).
Lower Bound: 0%


Increases or decreases in inflation should be proportional to the magnitude of
the difference between the actual staking participation rate and the target rate
(for example, 50% as per this proposal).


This approach would allow for a more dynamic response to fluctuations in staking
participation. By aligning inflation adjustments with the actual deviation,
network issuance better reflects the network’s real-time economic and security
conditions.


Inflation adjustment function:


Δi= k * Δs


Δi = Inflation change for the new epoch
k = Speed Co-efficient
Δs = Staking Participation Rate (s) at the start of epoch – Target Staking
Participation Rate (T)


inew = max (0%, min (current issuance curve, ilast + Δi)


ilast = Inflation in the last epoch
inew = Inflation in the new epoch
current issuance curve = inflation defined by current Solana issuance curve


This proposal sets k = 0.05 per annum. So, for each extra percentage point
higher/lower in staking participation rate, inflation would come down/go up by
0.05% p.a. in the next epoch. With the current staking participation rate of
~70%, the network would see a reduction of inflation of 1% p.a. in the next
epoch. On the other hand, with a hypothetical staking participation rate of say
40%, the network would see an increase of inflation of 0.5% p.a. in the next
epoch.


The max function ensures that inflation is at least zero, and the min function
ensures that the inflation does not rise above the current issuance curve.
This design offers several key benefits:
Consensus Safety: Adjusting inflation based on staking participation ensures
sufficient validator incentives to maintain network security, prioritizing
consensus safety.
Market-Based Flexibility: The model adapts to the network's economic activity,
making it more responsive to changing market conditions. It’s possible to
imagine a future where stakers are earning enough from MEV that no SOL emissions
are necessary.
Validator Retention: It accommodates Solana-aligned validators who are willing
to stake even with lower emissions, recognizing that they can earn more through
MEV in higher economic activity ecosystems.
This dynamic approach balances the need for a secure, decentralized network with
the flexibility to thrive in a competitive market.




## Alternatives Considered


### Alternative Design 1: Pick another fixed curve
A simple alternative would be to adjust the issuance rate to a fixed number,
determined by community inputs. However, this approach presents several risks:
Lack of Market Mechanisms: Setting a fixed rate ignores the dynamics of free
markets and the network’s real-time economic conditions.
Arbitrary Adjustments: Using another arbitrary number risks undermining the
integrity of the system and may lead to decisions that are disconnected from the
network’s needs.
Erosion of Trust: Relying on fixed adjustments could erode trust in the
community’s decision-making process, especially if future changes seem
disconnected from market realities.
Compromised Consensus Safety: A fixed issuance rate, especially in uncharted
territory, could undermine consensus safety, as it would not be dynamically tied
to staking participation or broader network health.


### Alternative Design 2: Fix Target Staking Yield
MEV has become a significant revenue source for stakers. One can consider
changing the issuance rate by factoring in MEV tips, maintaining the same
target yield as the original curve but offsetting it by the 30-day moving
average of MEV tips.
New Issuance Rate (i) = Target Staking Yield − 30-day moving average of MEV tips
MEV tips reflect real revenue for validators and stakers, allowing the system to
adjust to market conditions:
Hot Markets: Higher MEV tips allow for lower emissions.
Cold Markets: Increased emissions compensate validators, maintaining network
security.
This approach is inspired by central bank monetary policy, adjusting inflation
based on economic conditions.
But the big challenge with this design is that it incentivizes MEV payments to
move out of sight of the tracking mechanism, thereby rendering the design
completely ineffective.
For an abundance of clarity, we are not proposing any design which requires
measuring MEV payments.


## Impact


Implemented thoughtfully, this design could have a major positive economic
impact on the overall health of the Solana economy.


## Security Considerations


Targeting a staking participation rate of 50% ensures sufficient stake for
consensus safety while maintaining the network’s security and decentralization.


Below 33%, we potentially risk network safety because a supermajority of all SOL
has explicitly not voted on any given block and this opens the edge case
possibility of long range attacks. It is important to note that these long range
attacks are entirely theoretical and we have not seen one in practice. There are
other mechanisms in Solana to protect against long range attacks.


This proposal is the first in a series of steps to make Solana’s consensus more
secure and economics more market driven. The successor to this proposal is
another SIMD that introduces the concept of long-term staking, which seeks to
improve network security. The option to unstake SOL on a relatively short notice
(i.e., a short cool down period) poses a potential risk to networks’ stability
and safety, particularly in extreme circumstances where a significant amount of
SOL is unstaked within a brief timeframe. The combination of these two SIMDs
address these concerns while improving network security and economic activity.




