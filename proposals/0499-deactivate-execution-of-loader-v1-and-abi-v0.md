---
simd: '0499'
title: Deactivate execution of loader-v1 and ABI-v0
authors:
  - Alexander Meißner (Anza)
category: Standard
type: Core
status: Review
created: 2027-03-16
feature: TBD
---

## Summary

Deactivate execution of loader-v1 and ABI-v0.

## Motivation

See the related [SIMD discussion](https://github.com/solana-foundation/solana-improvement-documents/discussions/483).

The trouble with ABIv0 is that it had no alignment padding in its serialization
format and simply ignored any alignment requirements for syscall parameters.
This has since become undefined behavior in Rust. Disabling execution of
loader-v1 would allow us to remove ABIv0 entirely (as it is the only loader
which supports that ABI version) and would reduce the maintenance and auditing
burden in most syscalls significantly.

## New Terminology

None.

## Detailed Design

After the activation of the associated feature key a validator must fail to
execute programs owned by loader-v1, throwing the error message
`InstructionError::UnsupportedProgramId`.

## Alternatives Considered

- First bumping the CU cost of ABI-v0 significantly to get users off the
Memo-v1 program.
- Adapting the Memo Program v1 to ABI-v1 and redeploying it on loader-v3.

## Impact

The only loader-v1 / ABIv0 program still in use today is Memo Program v1.
Of which there has been a loader-v2 / ABIv1 replacement around for a long time.

## Security Considerations

The activation of this feature should be straight forward but the later clean
up together with the removal of alignment check skipping would be more complex.

## Drawbacks

Slight inconvenience to last remaining users of the Memo Program v1. However,
the issues this incurrs to validator implementations should outweigh this.
