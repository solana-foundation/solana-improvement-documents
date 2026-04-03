---
simd: '0430'
title: Relax Program Buffer Constraints
authors:
    - Dean Little (Blueshift)
    - Joe Caulfield (Anza)
category: Standard
type: Core
status: Review
created: 2025-12-18
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD proposes relaxing current constraints on program buffers used by the
`DeployWithMaxDataLen` (initial deployment) and `Upgrade` (redeployment)
instructions, currently requiring them to:

- Be owned by `BPFLoaderUpgradeab1e11111111111111111111111`, and
- Share an upgrade authority with the program being deployed or upgraded

## Motivation

Currently, it is not feasible to support several advanced use cases for 
program buffers, including:

- Sponsored deployments
- Permissionless buffer reuse
- Retracting erroneous upgrades using a common buffer
- On-chain recompilation

By removing these constraints, the loader can support a broader range of 
advanced and flexible deployment workflows.

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

The `DeployWithMaxDataLen` and `Upgrade` instructions will be updated to include
an optional `close_buffer` boolean input. If not provided, the default will
be `true`.

```
DeployWithMaxDataLen {
  max_data_len: u64,
  close_buffer: bool, // New
}
Upgrade {
  close_buffer: bool, // New
}
```

The accounts required by the instructions are unchanged, but the signer
requirements differ based on the value of the `close_buffer` option.

For a value of `true`, existing behavior is preserved. The buffer account will
be closed (lamports transferred to a designated recipient and account data
zeroed).

For a value of `false`, the buffer account is not modified, enabling reuse for
future deployments. Since the buffer is not closed, its lamports are not
transferred to the spill account. Instead, the program data account must
already contain sufficient lamports to satisfy rent requirements. In practice,
this means the deployer or upgrader must pre-fund the program data account
(e.g. via a transfer in a preceding instruction) before invoking the deploy or
upgrade. The net rent cost to the end user remains the same as in the
`close_buffer=true` case, since the user did not pay for the buffer.

Additionally, constraints on the buffer are relaxed:

- No buffer authority signature is required.
- No buffer ownership check is required.
- The `IncorrectAuthority` check is removed:
  - `DeployWithMaxDataLen`: The buffer's authority no longer must match the
    authority that will be set on the deployed program.
  - `Upgrade`: The buffer's authority no longer must match the upgrade
    authority stored on the program account.

Note that the program's authority account must still be provided in the same
position for both instructions and must still sign the transaction. Only the
buffer-related checks are relaxed for `close_buffer=false`; the
`MissingRequiredSignature` check for the program's authority remains enforced.

```
              DeployWithMaxDataLen / Upgrade { close_buffer }
                                    |
                        +-----------+-----------+
                        |                       |
                 close_buffer=true       close_buffer=false
                    (default)                   |
                        |               Relaxed buffer checks
                 Existing checks        Buffer not modified
                 Buffer closed          (reusable)
```

### Buffer Layout Requirement

Regardless of the buffer's owner, the buffer account must still conform to the
expected layout for Loader V3 buffer accounts. Specifically, the account data
must deserialize to `UpgradeableLoaderState::Buffer` (discriminant `1` as a
little-endian u32), and the ELF data must begin at the expected offset within
the account data (after the header).

```
| discriminant (4 bytes) | authority_address (33 bytes) | ELF data ... |
|       0x01000000       |    option byte + pubkey      |              |
|                        |                              |              |
|<--------------- 37-byte header ---------------------->|
```

## Alternatives Considered

- Introduce a new loader that enables these behaviors by default
- Only relax the authority constraint (similar outcome, but requires CPI)
- Retain existing constraints and make no changes

## Impact

This change enables more flexible program deployment workflows, unlocking new 
use cases for developers. Some examples include:

- **Emergency abort programs**: Protocols under attack can deploy an
  [emergency abort program][sbpf-asm-abort] from a pre-staged buffer to
  immediately halt all flows through their program, without needing to prepare
  a buffer during the incident.
- **Verified open-source deployments**: Developers can publish verified builds
  of shared open-source program implementations as reusable buffers, allowing
  users who wish to fork the program to deploy it directly without building
  and uploading the binary themselves.
- **Education and onboarding**: Education centers can make buffers available
  for programs used in lesson plans, so students can easily deploy necessary
  components to test their implementations on higher networks (e.g. devnet,
  mainnet-beta).

[sbpf-asm-abort]: https://github.com/deanmlittle/sbpf-asm-abort

## Security Considerations

This proposal introduces two new, strictly opt-in potential attack vectors:

1. In multisig deployment flows (e.g., Squads), if a quorum authorizes an 
upgrade using a buffer it does not own, the buffer could be modified by a 
third party prior to deployment. This introduces additional security 
considerations beyond those enforced by the multisig quorum itself.
2. If a buffer is owned by a third-party program, that program may retain 
write access to the buffer account irrespective of its upgrade authority. 
This creates a potential supply chain attack vector if the security 
assumptions of the owner program are not carefully evaluated.

## Backwards Compatibility

This feature relaxes existing Loader V3 constraints and is therefore not 
backwards compatible for consensus, necessitating a feature gate. For CLI and 
tooling, the change is fully backwards compatible.
