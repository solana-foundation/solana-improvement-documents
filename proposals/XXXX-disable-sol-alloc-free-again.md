---
simd: '0706'
title: Disable the `sol_alloc_free_` syscall for new deployments again
authors:
  - Simonas Kazlauskas (Anza)
category: Standard
type: Core
created: 2026-10-26
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Disable `sol_alloc_free_` syscall for new deployments again. Additionally, increase the CU cost of
this syscall to further discourage any remaining use.

## Motivation

This feature is meant to address an oversight. Any program referencing `sol_alloc_free_`
was rejected at deployment since 2022. However, with a switch to static syscalls, this filter is no
longer effective and programs invoking the `sol_alloc_free_` syscall may be deployed again.

Modern programs bring their own allocator, making this syscall a distraction to everyone involved.

## Related SIMDs

This proposal is related to the following previously accepted proposal:

- **[SIMD-0178]: SBPF Static Syscalls** which exposed the defect in implementation.
- **[SIMD-0500]: Disable deployment of SBPF-v0, -v1 and -v2**.

[SIMD-0178]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0178-static-syscalls.md
[SIMD-0500]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0500-disable-deployment-of-sbpf-v0-v1-v2.md

## Detailed Design

Upon deployment or upgrade of any program, programs containing a static syscall instruction to
`sol_alloc_free_` (`CALL_IMM 0x83F00E8F`) MUST be rejected the same way as any other program
invoking an unknown syscall.

Any preexisting programs that invoke the `sol_alloc_free_` syscall now have to pay an additional
`syscall_base_cost` CUs for each invocation of this syscall to encourage programs from migrating
away from this syscall and to use their own allocators.

## Impact

This change is expected to be effectively transparent to the users. The syscall had been disabled
for the past 4 years and only recently got re-enabled. In the corpus of programs deployed on
mainnet there are no SBPFv3 programs that reference this syscall. Out of 209 <=v2 programs that
may call the `sol_alloc_free` syscall,

## Conformance

A test that attempts to deploy a SBPFv3 program invoking the `sol_alloc_free_` syscall should
suffice. It could be a conformance test suite fixture.

## Alternatives considered

Remove the syscall altogether – would break 209, a couple of which are still being actively used,
most notably Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo.

## Security considerations

None
