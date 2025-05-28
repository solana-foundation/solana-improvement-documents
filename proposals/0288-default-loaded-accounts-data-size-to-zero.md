---
simd: '0288'
title: Set default value of loaded_accounts_bytes to zero 
Author: Tao Zhu (Anza)
category: Standard
type: Core
status: Draft
created: 2025-05-27
feature: 
supersedes: 
superseded-by:
extends:
---

## Summary

This SIMD proposes setting the default value for loaded accounts data size to
**zero bytes** in the Solana runtime, replacing the current implicit default of
`MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES` (64MB).

## Motivation

Currently, the Solana runtime allows transactions to implicitly load up to 64MB
of account data:

```rust
/// The total accounts data a transaction can load is limited to 64MiB to not break
/// anyone in Mainnet-beta today. It can be set by set_loaded_accounts_data_size_limit instruction
pub const MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES: NonZeroU32 =
    NonZeroU32::new(64 * 1024 * 1024).unwrap();
```

This generous default was intended to avoid accidental transaction failures
during development or early deployment phases. However, it introduces several
downsides:

- Reduces transparency in runtime constraints.

- Enables unintentional or excessive resource usage.

- Increases the risk of performance degradation or abuse.

By reducing the default to zero, developers and operators are required to
explicitly configure this budget, leading to safer, more predictable, and
better-controlled execution environments.

This aligns with broader Solana design goals to ensure deterministic resource
consumption and to encourage clear contract behavior.

## Alternative Considered

- Reduce the default to a arbitrary value (e.g., 8MB or 16MB) instead of 0.

## New Terminology

None

## Detailed Design

1. Introduce a New Default Constant:
   Add a new constant:`pub const DEFAULT_LOADED_ACCOUNTS_DATA_SIZE_BYTES: usize = 0`
   Use this wherever a default loaded account data size is required.

2. Preserve the Current Maximum:
   Continue to enforce `MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES` (64MB) as the upper
bound for explicitly configured limits.

3. Gradual Ramp-Down Strategy:
   Implement a phased reduction toward zero. For example, decrease
`DEFAULT_LOADED_ACCOUNTS_DATA_SIZE_BYTES` by fixed increments (e.g., 8MB per
epoch). This gives developers time to adapt.

4. Final Enforcement:
   Once the target default of zero is reached, completely remove the default.
All accounts data size limits must then be explicitly defined by the transaction or runtime.

## Impact

This change is not backward-compatible by default. Workloads or tests that
rely on the implicit 64MB limit may break.

To mitigate the impact:

- Issue deprecation warnings during the ramp-down phase.

- Provide guidance and tools for setting explicit limits using the
set_loaded_accounts_data_size_limit instruction.

- Use feature gating to manage rollout and allow clusters to opt in progressively.


## Security Considerations

To maintain consensus integrity, both Agave and Firedancer clients must adopt
this change in a coordinated fashion.
