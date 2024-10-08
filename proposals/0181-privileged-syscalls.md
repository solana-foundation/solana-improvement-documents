---
simd: '0181'
title: Privileged Syscalls
authors:
  - Joe Caulfield | Anza
category: Standard
type: Core
status: Draft
created: 2024-10-02
feature: (fill in with feature tracking issues once accepted)
supersedes: (optional - fill this in if the SIMD supersedes a previous SIMD)
superseded-by: (optional - fill this in if the SIMD is superseded by a subsequent
 SIMD)
extends: (optional - fill this in if the SIMD extends the design of a previous
 SIMD)
---

## Summary

Support for privileged syscalls, to perform special-cased runtime activities,
for enshrined program IDs only.

## Motivation

As we continue the effort to migrate all builtin programs to on-chain BPF
programs (Core BPF), the ability for some of these programs to perform
privileged operations becomes immediately relevant.

Some examples of such operations are:

- Allocating very large accounts (System program)
- Marking accounts as `executable` (loaders)

In order to accomplish the aforementioned goal, some programs - such as System
and loaders - will need the ability to perform activites that are not permitted
to all on-chain BPF programs.

Similar to the purpose of [SIMD 0088](./0088-enable-core-bpf-programs.md), this
SIMD serves to establish the existence of "privileged syscalls", which are
syscalls only available to enshrined programs.

## New Terminology

- **Privileged Syscall**: A virtual machine builtin function only available to
  runtime-enshrined program IDs.

## Detailed Design

The Solana Virtual Machine (SVM) follows a well-defined Instruction Set
Architecture (ISA) for supported VM op codes. Additionally, the Solana protocol
also dictates a set of interfaces for VM builtin functions known as System
Calls, or "syscalls".

All of the protocol-defined syscalls are made available to all on-chain programs
through a loader, which implements each syscall interface at the runtime level,
allowing on-chain programs to call into them to perform certain actions, such as
logging and invoking other programs.

This proposal suggests adding interfaces for "privileged" syscall interfaces to
the Solana protocol. These syscalls would specifically _not_ be made available
to all on-chain programs, but rather a subset of programs, represented by an
enshrined set of program IDs within the runtime.

These privileged syscalls must _only_ be registered VM builtin functions for
enshrined programs. When the runtime encounters an enshrined program to be
executed, it must register the necessary privileged syscalls as VM builtin
functions for the provisioned VM instance. When any non-enshrined program is
encountered, these functions must not be registered.

Each new privileged syscall introduced to the protocol must have its own SIMD.
The program IDs that must be granted access to that particular privileged
syscall must be included in the proposal.

## Alternatives Considered

The primary alternative to privileged syscalls is for any builtin programs that
perform privileged operations to remain builtins, and not be migrated to Core
BPF. However, this would force validator client teams to maintain these builtins
with their clients.

With the suggested approach for privileged syscalls, validator client teams
would instead only have to maintain these syscalls themselves, not entire
programs.

## Impact

As mentioned above, privileged syscalls would further enable all builtin
programs to be migrated to Core BPF, reducing the maintenance burdern for
core contributors from various validator client teams.

Developers and validators are unaffected. All on-chain programs that are not
granted access to privileged syscalls are unaffected. Those programs that are
granted access would be backwards compatible.

## Security Considerations

The primary security consideration is ensuring that only those enshrined
programs can access these privileged syscalls. If not implemented correctly,
core contributors could mistakenly grant privileged abilities to ordinary
on-chain programs, which could have consequential implications on consensus.

