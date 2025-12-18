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
Loader V3 to the program's upgrade authority.

## Motivation

Currently, due to the permissionless nature of the extend program instruction 
and some complexities surrounding the program cache, there is a DoS vector by 
which anyone could disable a program for one slot by permissionlessly 
extending its program data account.

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

1. Add a check to the extend program instruction to ensure it is being invoked 
by the current program upgrade authority.
2. Activate this change with a feature gate.
3. Remove feature gate after network activation.

## Alternatives Considered

- Allow DoS vectory to remain unresolved

## Impact

This proposal will remove the DoS vector for all deployed programs. Due to 
constraints of ABI V1, in the case that a multisig upgrade authority wishes to 
extend the program data account by greater than 10KiB, it will either need to 
create multiple resize proposals, or atomically set its authority to a 
top-level signer and reclaim it in the same transaction.

## Security Considerations

In the case of a multisig atomically setting its authority to a top-level 
signer, it is important to introspect the transaction and ensure that it 
consists of the following instructions:

- Set upgrade authority to top-level signer
- Extend program data account in top-level instruction
- Set upgrade authority back to quorum

If this order is not observed, it would be possible for a quorum to 
accidentally lose its upgrade authority.

## Backwards Compatibility

This feature places additional restrictions upon an existing Loader V3 
instruction and is therefore not backwards compatible, necessitating a feature gate.