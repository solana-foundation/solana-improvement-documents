---
simd: '0130'
title: Dynamic Block Limits
authors:
  - Cavey Cool, Magnetar Fields
category: Standard
type: Core
status: Draft
created: 2024-03-24
feature: (fill in with feature tracking issues once accepted)
supersedes: (optional - fill this in if the SIMD supersedes a previous SIMD)
extends: (optional - fill this in if the SIMD extends the design of a previous
 SIMD)
---

## Summary

This proposal introduces dynamic adjustments to the compute unit (CU) limit of
Solana blocks based on network utilization at the end of each epoch. If the
average block utilization exceeds 75%, the CU limit will increase by 20%;
if it falls below 25%, the limit will decrease by 20%. A second metric based on
vote slot latency is used to preserve protocol liveness and responsive UX. This
proposal aims to optimize network performance by adapting to demonstrated
compute capacity and demand without centralized decisions about limits and
without voting.

## Motivation

The goal of this proposal is to enhance the scalability and efficiency of the
Solana network by dynamically adjusting block CU limits in response to actual
network demand, ensuring Solana scales with Moore's Law and exponentially
increasing bandwidth.

## Alternatives Considered

**Manual Adjustment**: Periodic manual reviews of network performance to adjust
CU limits. This lacks responsiveness, flexibility, and may be an arbitrary or
centralized decision.

## New Terminology

- **Dynamic Block Limit**: Adjusting block CU limits based on network demand.
- **Epoch Average Utilization**: The average CU utilization over an epoch.
- **CU Limit Adjustment**: The process of modifying the block's CU limit.
- **Supermajority Vote Slot Latency**: 67th percentile for the vote slot latency
in an epoch.

## Detailed Design

### Implementation

Record each block's CU utilization in an epoch and calculate the epoch average
utilization (EAU). Record the slot latency of votes in an epoch and calculate the
supermajority vote slot latency (SVSL).

Increase CU limit by 20% if:

1. EAU is greater than 75%, **AND**
2. SVSL is below threshold vote slot latency (TBD).

Decrease CU limit by 20% if:

1. EAU is less than 25%, **OR**
2. SVSL is above threshold vote slot latency (TBD).

It is worth discussing the reasoning for the two criteria for increasing the CU
limit. Since the block schedule is determined by stake, the first metric (EAU)
serves as a stake-weighted metric of block utilization. This is a indicator of
true current demand and capacity; the EAU can only be observed to be above the
threshold if the network demonstrates that it can handle such compute and that there
is demand for it. **It is important to not increase CU limits if there is no demand
for it, as arbitrarily increasing the CU limit opens up a vector for validators
to produce fat blocks that the rest of the network may struggle to replay.**

The second metric is present to preserve protocol liveness and responsive UX. If
the SVSL is above the target threshold, it means that there are nodes in the
supermajority that are struggling to replay and vote within the target latency.
This threatens protocol liveness, as it can be an indicator of nodes within the
supermajority not being able to catch up to the tip. Furthermore, vote slot
latency is directly tied to the solana user experience; it can serve as a proxy
metric for finalization time.

Thus, the increase criteria can be summarized as follows: increase capacity if
there is demonstrated capacity and demand for it **AND** if it does not threaten
protocol liveness or degrade UX. Similarly , the decrease criteria can be
summarized as follows: decrease capacity if there is no demonstrated demand for
it **OR** if the current limits are threatening protocol liveness or degrading UX.

By setting thresholds at 75% and 25%, this proposal very roughly targets a 50%
block utilization. It is important to recognize that validator hardware is
inhomogeneous, and block producing capacity will vary from node to node. If the
average utilization is too high, it means that the protocol is throttling nodes
with high end hardware that could be producing more profitable blocks. If the average
utilization is too low, then it means that the limits are either too high or there
is no demonstrated demand for blockspace. By roughly targeting 50% utilization,
and with the aforementioned checks in place, nodes with higher end hardware are
allowed to produce slightly larger blocks than other nodes without threatening
protocol liveness.

**Safety Measures**: Implement maximum and minimum CU limits to prevent extreme adjustments.

### Interaction and Integration

- Fits into the Solana runtime as a dynamic parameter adjustment mechanism.
(Perhaps in the future, other constant parameters can be made dynamic using a
similar mechanism.)
- Will require updates to how block capacity is calculated and adjusted at the
epoch level.

## Impact

**Validators**: Revenue capacity will increase naturally with better hardware,
incentivizing validators to upgrade collectively. The lowest performing validators
will eventually be forced toward better hardware and more bandwidth.

**Core Contributors**: Need to consider implications on network stability and
performance monitoring.

## Security Considerations

Implementing dynamic block limits must not introduce vectors for DoS attacks or
manipulation of block capacity. Thorough testing, simulations, and historical
analyses are required to ensure the mechanism's resilience against such threats.

## Drawbacks

**Pros**:

- Offers real-time adjustment according to demonstrated network capacity.
- Directly targets network demand and capacity issues.

**Cons**:

- It may take several epochs to adjust to a large step function increase in demand.
- Introducing dynamic block limits adds complexity to the network's operation/monitoring.

## Backwards Compatibility

- The proposal requires network-wide coordination for activation.
