---
simd: '0444'
title: Relax program data account check in migration
authors:
  - febo (Anza)
category: Standard
type: Core
status: Review
created: 2026-01-09
feature: rexav5eNTUSNT1K2N7cfRjnthwhcP5BC25v2tA4rW4h
---

## Summary

This proposal relaxes the requirements for runtime-level migrations of programs
(such as builtins) to permit system-owned programdata accounts to hold lamports.

## Motivation

Currently, runtime-level program migrations require that the target program
data account must not already exist. This means the account must have zero
lamports.  If the program data account is pre-funded (i.e., has a lamports
balance greater than `0`), the migration will not proceed.

## Alternatives Considered

Taking no action will result in a situation in which pre-funding a target
program data account prevents the migration from being completed.

## New Terminology

N/A

## Detailed Design

Improve the check for target program data accounts to relax the requirement
that they must not exist. If an account with the derived address for the target
program data account already exists, holds lamports, and is owned by the system
program, the migration proceeds.

Current logic (pseudo-code):

```rust
// The program data account should not exist and have zero lamports.
if bank.get_account(&program_data_address).is_some() {
    // --> return error
}
```

Logic after this SIMD is enabled:

```rust
// The program data account should not exist, but a system account with funded
// lamports is acceptable.
if let Some(account) = bank.get_account(&program_data_address) {
    if account.owner() != &SYSTEM_PROGRAM_ID {
        // --> return error
    }
}
```

Note that an ownership check is sufficient, since allocating a Loader V3
programdata PDA under System is not feasible.

Important Note: If the system-owned programdata account contains lamports,
those lamports must be burned during any runtime-level program migrations
by updating the Bank's capitalization.

## Impact

This change prevents the migration from failing when the target program
data account is pre-funded.

## Security Considerations

N/A

## Backwards Compatibility

This proposal itself does not introduce any breaking changes.
