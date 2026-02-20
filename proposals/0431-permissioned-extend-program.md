---
simd: '0431'
title: Permissioned Extend Program
authors:
    - Dean Little (Blueshift)
    - Joe Caulfield (Anza)
category: Standard
type: Core
status: Review
created: 2025-12-14
feature: (fill in with feature key and github tracking issues once accepted)
supersedes: '0164'
---

## Summary

This SIMD proposes restricting invocation of the extend program instruction in 
Loader V3 to the program's upgrade authority, along with lifting the current
restriction preventing it from being invoked via CPI.

## Motivation

Currently, due to the permissionless nature of the extend program instruction 
and some complexities surrounding the program cache, there is a DoS vector by 
which anyone could disable a program for one slot by permissionlessly 
extending its program data account. Thus the motivation of this SIMD is to both
resolve the DoS vector by restricting access to this instruction to the 
program's upgrade authority, whilst improving the devex of this new 
restriction by allowing ExtendProgram to be invoked via CPI.

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

The `ExtendProgram` instruction will require the program's upgrade authority as
a signer and it will be available for invocation via CPI.

### Changes to Required Accounts

The current `ExtendProgram` instruction expects the following accounts:

```
0. [w] ProgramData account
1. [w] Program account
2. [ ] System program, optional
3. [ws] Payer, optional
```

After this proposal's feature gate is activated, the instruction will expect:

```
0. [w] ProgramData account
1. [w] Program account
2. [s] Upgrade authority    // New
3. [ ] System program, optional
4. [ws] Payer, optional
```

### Control Flow

The instruction will verify:

1. The program has an upgrade authority set (i.e., is not immutable). If not,
   return `Immutable`.
2. The provided authority matches the program's stored upgrade authority. If
   not, return `IncorrectAuthority`.
3. The authority account is a signer. If not, return
   `MissingRequiredSignature`.

### CPI Restriction Removal

The current restriction preventing `ExtendProgram` from being invoked via CPI
will be removed. The instruction will be fully available for CPI.

## Alternatives Considered

- Allow DoS vector to remain unresolved
- Retain existing account ordering by combining payer and authority into a
  single mandatory account

## Impact

This proposal will remove the DoS vector for all deployed programs. Due to 
constraints of ABI V1, in the case that a multisig upgrade authority wishes to 
extend the program data account by greater than 10KiB, it will either need to 
create multiple resize proposals, or atomically set its authority to a 
top-level signer and reclaim it in the same transaction. The `ExtendProgram`
instruction will now also be invokable by CPI.

## Security Considerations

In the case of a multisig atomically setting its authority to a top-level 
signer, it is important to introspect the transaction and ensure that it 
executes the following behavior:

- Set upgrade authority to top-level signer
- Extend program data account in top-level instruction
- Set upgrade authority back to quorum

If this behavior is not observed, it would be possible for a quorum to 
accidentally lose its upgrade authority.

## Backwards Compatibility

This feature places additional restrictions upon an existing Loader V3 
instruction and is therefore not backwards compatible, necessitating a feature gate.
