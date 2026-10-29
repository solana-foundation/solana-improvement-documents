---
simd: '0706'
title: Disable the `sol_alloc_free_` syscall again
authors:
  - Simonas Kazlauskas (Anza)
category: Standard
status: Review
type: Core
created: 2026-10-26
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Disable `sol_alloc_free_` syscall for new deployments again. Additionally,
increase the CU cost of this syscall to further discourage any remaining use.

## Motivation

This feature is meant to address an oversight. Any program referencing
`sol_alloc_free_` was rejected at deployment since 2022. However, with a switch
to static syscalls, this filter is no longer effective and programs invoking
the `sol_alloc_free_` syscall may be deployed again.

Modern programs bring their own allocator, making this syscall a distraction to
everyone involved.

## Related SIMDs

This proposal is related to the following previously accepted proposal:

- **[SIMD-0178]: SBPF Static Syscalls** which exposed the defect in the
  implementation.

[SIMD-0178]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0178-static-syscalls.md

## Detailed Design

Upon deployment or upgrade of any program, programs containing a static syscall
instruction to `sol_alloc_free_` (`CALL_IMM 0x83F00E8F`) MUST be rejected the
same way as any other program invoking an unknown syscall.

Any preexisting programs that invoke the `sol_alloc_free_` syscall now have to
pay an additional `syscall_base_cost` CUs for each invocation of this syscall
to encourage programs to migrate away from this syscall and use their own
allocators.

## Impact

This change is expected to be effectively transparent to most users. The
syscall was disabled for the past 4 years and was only recently re-enabled. In
the corpus of programs deployed on mainnet there are no SBPFv3 programs that
reference this syscall. Out of 209 <=v2 programs that may call the
`sol_alloc_free_` syscall, only one is used heavily:
[`Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo`][memo]. Only three other
programs ([1][p1], [2][p2], [3][p3]) had any use in the past month, all of
which are upgradeable. The remainder had no use within the past 3 months.

[memo]: https://explorer.solana.com/address/Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo
[p1]: https://explorer.solana.com/address/95o7H8qXc57fgfwp8ziyYue6iMRxrBZDLrcSaCntc4K1
[p2]: https://explorer.solana.com/address/B1hM7wsknuk35msAgVc9zJyc1HidceZgmK39xGafgKR4
[p3]: https://explorer.solana.com/address/CpbvNz3bePtuRPaNJ46UuEMFaJdfjTuMf8d92AFXxjsB

`Memo` program's syscall usage scales with the number of accounts specified
in the instruction to the order of $$4n+2, n ≥ 1$$. A sampling of recent
transactions shows that 75% of instructions specify no accounts and the CU
count remains unchanged. The remainder pass in 1 account, increasing the CU use
for those instructions by 600.

The `4n+2` syscall calls come from the SDK's entrypoint rather than program
code itself, so a similar effect can be anticipated for all 209 programs.

## Conformance

A test that attempts to deploy a SBPFv3 program invoking the `sol_alloc_free_`
syscall should suffice. It could be a conformance test suite fixture.

## Alternatives Considered

Remove the syscall altogether – would break 209 programs, a couple of which are
still being actively used.

Charge CU only when `free_addr == 0` (allocations,) or a smaller amount per
call.

## Security Considerations

None

## New Terminology

None
