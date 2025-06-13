---
simd: '0268'
title: Raise CPI Nesting Limit
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Accepted
created: 2025-03-26
feature: 6TkHkRmP7JZy1fdM6fg5uXn76wChQBWGokHBJzrLB3mj
extends: SIMD-0219
---

## Summary

Increase the maximum number of nested CPI calls.

## Motivation

The complexity of dApp interoperation is limited by how many programs can call
into one another.

## New Terminology

None.

## Detailed Design

Once the associated feature gate is activated the maximum nesting depth of CPI
calls must be changed from 4 to 8. This feature should only be activated after
SIMD-0219.

## Alternatives Considered

None.

## Impact

Exisiting dApps will not be affected as long as they don't depend on this limit
in their logic to fail transactions, which is unlikely.

## Security Considerations

None.

## Drawbacks

The maximum amount of VMs stack and heap memory, which needs to be reserved and
zeroed out, would double.
