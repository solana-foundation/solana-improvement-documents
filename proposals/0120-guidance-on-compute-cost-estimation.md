---
simd: '0120'
title: Guidance on compute cost estimation
authors:
  - Tao Zhu (Anza)
category: Standard
type: Core
status: Draft
created: 2024-02-29
feature: (fill in with feature tracking issues once accepted)
supersedes: (optional - fill this in if the SIMD supersedes a previous SIMD)
extends: (optional - fill this in if the SIMD extends the design of a previous
 SIMD)
---

## Summary

Recommends procedure to measure then estimate compute cost for programs.

## Motivation

In Solana, there is a requirement for built-in and precompiled programs to have
a flat cost. Establishing a recommended procedure for assessing and determining
the computational expenses incurred during the development and deployment of
these programs can be beneficial.

## Alternatives Considered

N/A

## New Terminology

None

## Detailed Design

Recommended Procedure for Measuring and Estimating Built-in and/or Precompiled
Programs:

1. Benchmark Program:

  - Ensure that the code for benchmarking is reviewed and committed,
    facilitating future benchmarking processes or utilization on different
    hardware configurations.
  - Conduct benchmarks on hardware setups that closely resemble the average
    configuration of the cluster, ensuring relevance.
  - If the performance of the program is influenced by varying inputs (such as
    data length), conduct benchmarks using a range of inputs within feasible
    parameters.
  - Express benchmark results in microseconds or nanoseconds.

2. Validation:

Whenever possible, validate benchmark results against metrics obtained from
testnet or mainnet-beta environments.

3. Cost Estimation:

  - Adopt a conservative yet realistic approach to estimating costs.
  - Typically, this involves considering worst-case performance scenarios with
    realistic inputs.

4. Defining Constant Cost:

Define a constant cost for the program as COMPUTE_UNIT_TO_US_RATIO * microsecond.

## Impact

Owners and developers of built-in and precompiled programs are advised to
adhere to the recommended procedure for estimating compute costs. It is
recommended to periodically rerun this procedure on deployed programs,
especially following hardware or software upgrades. If the computed costs have
significantly changed, owners and developers should update the program costs
accordingly and activate feature gates as necessary.

## Security Considerations

None

## Drawbacks *(Optional)*

None

## Backwards Compatibility *(Optional)*

None
