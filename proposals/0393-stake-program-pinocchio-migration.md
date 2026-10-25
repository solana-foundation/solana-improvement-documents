---
simd: "0393"
title: "Stake program Pinocchio migration (p-stake)"
authors:
  - Gabe (Anza)
  - Pete (Anza)
  - Febo (Anza)
  - Jon (Anza)
category: Standard
type: Core
status: Idea
created: 2025-11-06
feature: (to be assigned upon acceptance)
---

## Summary

This proposal outlines a plan to replace the current Solana Stake Program
(`Stake11111111111111111111111111111111111111`) with a highly efficient,
Pinocchio-based implementation named `p-stake`. The new program will be a
`no_std`, byte-for-byte ABI compatible, upstream eBPF toolchain-friendly,
drop-in replacement for the existing program.

## Motivation

The Stake program is one of the most fundamental programs in the Solana
ecosystem, used for delegating stake to validators and managing staking
rewards. Improvements to this program are far-reaching and are driven by two
key objectives:

- **Significant compute unit (CU) reduction**. Based on the success of the
  [p-token migration](https://github.com/solana-foundation/solana-improvement-documents/pull/266),
  which saw a +90% CU reduction, similar gains are anticipated for the Stake program.
  This improvement will free up blockspace, enhance composability for protocols
  CPI'ing in, and lower transaction fees for all users.
- **Aligning with upstream eBPF for future-proofing**. There is growing
  interest among Solana contributors in adopting compatibility with upstream
  eBPF (the standard implementation used in Linux kernel and Rust's toolchain).
  By moving to a `no_std` environment, p-stake prepares for this transition. It
  also positions the program well for meaningful feature development after the
  migration (additional features prior to the migration would mean double the
  work).

## Primary References

This implementation follows the patterns established by:

- [p-token](https://github.com/solana-program/token/tree/program%40v9.0.0/pinocchio):
  Token program Pinocchio migration
- [p-ata](https://github.com/solana-program/associated-token-account/pull/102):
  Associated Token Account program Pinocchio migration

## Impact

Benchmarks below are aggregated from the current Mollusk and program-test
runs. For each instruction, it averages all successful `CU_USAGE` samples
emitted by the harness and rounds the result down to the nearest compute unit.
As the migration progresses, the benchmarks will be published here. Expecting
+90% CU reductions.

| Instruction              | current program | p-stake |
|--------------------------|-----------------|---------|
| Initialize               | 7485            | -       |
| Authorize                | 10144           | -       |
| DelegateStake            | 13169           | -       |
| Split                    | 12029           | -       |
| Withdraw                 | 5941            | -       |
| Deactivate               | 10499           | -       |
| SetLockup                | 10770           | -       |
| Merge                    | 21082           | -       |
| AuthorizeWithSeed        | 10680           | -       |
| InitializeChecked        | 5219            | -       |
| AuthorizeChecked         | 9183            | -       |
| AuthorizeCheckedWithSeed | 10489           | -       |
| SetLockupChecked         | 9148            | -       |
| GetMinimumDelegation     | 669             | -       |
| DeactivateDelinquent     | 16076           | -       |
| MoveStake                | 19791           | -       |
| MoveLamports             | 12637           | -       |

- High variance instructions: `Merge` (min to max delta: 45,471 CU) and `Split`
  (min to max delta: 13,989 CU) show significant variance depending on stake
  account states, activation status, and whether accounts need reallocation.

## Detailed Design

The implementation maintains strict compatibility with the existing Stake
program:

- **Program ID**: Must not change
  (`Stake11111111111111111111111111111111111111`)
- **Account layouts**: Byte-for-byte compatible, including the `StakeStateV2`
  variants
- **Instruction Interface**: Discriminants, required accounts, and signer
  statuses will be identical.
- **Semantics**: Error codes, return data, and all state transitions will be
  exactly the same.

## External Dependency

[SIMD-0391](https://github.com/solana-foundation/solana-improvement-documents/pull/391):
Replaces floating point (not upstream ebpf friendly) stake warmup and cooldown
logic with fixed-point arithmetic. This is a prerequisite for porting
instructions involving stake calculations.

## Implementation Strategy

1. State migration to zero-copy

All on-chain state will be migrated to zero-copy views by defining structs with
`#[repr(C)]`. To guarantee ABI compatibility, a suite of golden tests or
compile-time size assertions will enforce:

- The total size of StakeStateV2 remains 200 bytes
- The u32 discriminant and variant order for the enum is preserved
- All nested struct layouts match the legacy implementation byte-for-byte

2. Instruction Migrations

Each of the Stake program instructions will be ported to Pinocchio handlers.
Each instruction migration will include additional unit tests (if coverage holes
are found w/ existing suite) and CU benchmarks.

3. Log removal

Following the precedent set by `p-token`, omitting logs saves ~100 CU per
instruction. They are not consensus-relevant, can be reconstructed from
transaction data by indexers, and are not reliable (may be truncated/spoofed).
That said, in error cases, logging is still quite useful in understanding the
nature of the failure. So we'll only retain logs relating to errors.

## Security Considerations

- **Comprehensive Testing**: All existing test fixtures will be ported and
  passed. A differential testing framework will execute instructions against
  both program versions and assert that the resulting state and status are
  identical.
- **Fuzzing**: The program will be heavily fuzzed by replaying historical
  mainnet transactions. Working with Firedancer to source fixtures.
- **External Security Audit**: The final implementation will undergo at least
  one comprehensive external security audit.
- **Formal Verification**: Employ formal methods to prove that p-stake
  demonstrates semantic equivalence with the legacy implementation.

## New Terminology

No new terminology is introduced with this proposal.

## Alternatives Considered

- Introduce new stake program ID. Requires ecosystem-wide, opt-in migrations
  with long tail fragmentation of tooling.
- Incremental optimization of existing code. Would be less disruptive but
  unable to achieve the dramatic CU reductions seen with Pinocchio.
- Keep full logging. Maintain CU overhead that can be reconstructed by indexers
  anyway.

The chosen approach balances immediate CU benefits with future upstream ebpf compatibility.

## Development Tracking

Tracking issue with roadmap and status will be published here when ready.
