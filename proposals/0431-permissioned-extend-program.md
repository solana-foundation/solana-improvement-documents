---
simd: '0431'
title: 'Loader V3: Minimum Extend Program Size'
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

Enforce a minimum extension size of 10,240 bytes (10 KiB) on the
`ExtendProgram` instruction in Loader V3, mitigating the existing
denial-of-service vector through economic deterrence while preserving the
instruction's permissionless nature.

## Motivation

The `ExtendProgram` instruction is currently permissionless — anyone can extend
any upgradeable program's data account by as little as 1 byte. Due to
complexities surrounding the program cache, each invocation of `ExtendProgram`
invalidates the program's cache entry for the current slot, effectively
disabling the program for one slot. Combined with the negligible cost of a
1-byte extension, this creates a cheap denial-of-service vector.

SIMD-0164 and earlier revisions of this SIMD attempted to fix this by making
`ExtendProgram` permissioned, requiring the upgrade authority as a signer.
While this provides absolute DoS protection, it breaks several important
workflows:

- **Multisig PDA authorities** cannot sign top-level instructions. With a CPI
  resize cap of 10 KiB per instruction, large extensions require multiple
  proposals or a clunky authority-shuffle pattern.
- **Self-upgrading programs** that manage their own upgrade authority as a PDA
  would lose the ability to extend themselves.

SIMD-0164 was never approved, and the general sentiment favored a more
flexible solution that preserves the permissionless nature of `ExtendProgram`.
As [suggested by jstarry][jstarry_suggestion], a minimum extension size achieves
this.

[jstarry_suggestion]: https://github.com/solana-foundation/solana-improvement-documents/pull/164#issuecomment-3138353713

A minimum extension size solves the DoS vector economically instead: at 10
KiB, each extension costs the attacker approximately 0.072 SOL in rent-exempt
lamports, which are irrecoverably donated to the victim's program data account.
Ten successive attacks cost the attacker 0.72 SOL while only benefiting the
program owner. This makes sustained griefing economically irrational without
breaking any existing workflows.

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

After this proposal's feature gate is activated, the `ExtendProgram`
instruction will enforce a minimum extension size of 10,240 bytes (10 KiB).

### Instruction Accounts

The instruction accounts remain unchanged:

```
0. [w] ProgramData account
1. [w] Program account
2. [ ] System program, optional
3. [ws] Payer, optional
```

### Control Flow

The instruction will verify the following, in addition to all existing checks:

1. The requested extension size is at least 10,240 bytes. If not, return
   `InvalidArgument`.

All other existing checks (program ownership, account state, rent-exempt
balance) remain unchanged.

### CPI Restriction

The existing restriction preventing `ExtendProgram` from being invoked via CPI
is not modified by this proposal.

## Alternatives Considered

### Permissioned ExtendProgram (SIMD-0164)

Require the upgrade authority as a signer and lift the CPI restriction. This
provides absolute DoS protection but breaks multisig PDA workflows,
self-upgrading programs, and any third-party tooling that extends programs on
behalf of owners.

### Disable ExtendProgram entirely

If `ExtendProgram` were made permissioned, there would be little reason for it
to exist as a standalone instruction — resizing could instead be folded into
`Upgrade`. However, keeping `ExtendProgram` permissionless avoids nasty
workarounds for multisigs and self-upgrading programs: anyone can crank a
top-level extend to prime up space before a multisig upgrade, without needing
the multisig authority to sign. It also sidesteps the CPI 10 KiB
CPI growth limit that would otherwise cap how much a program can extend itself
in a single call.

### Smaller minimum extension size

A 1 KiB minimum costs only ~0.008 SOL per attack — too cheap to deter
sustained griefing.

### Larger minimum extension size

Minimums of 20 KiB or 50 KiB provide stronger deterrence but
disproportionately affect small programs. A survey of 14,822 mainnet-beta
programs shows the following size distribution:

| Size Range   | Programs | Share  |
|--------------|----------|--------|
| 0 – 10 KiB  | 149      | 1.0%   |
| 10 – 50 KiB | 843      | 5.7%   |
| 50 – 200 KiB| 2,066    | 13.9%  |
| 200 – 500 KiB| 6,058   | 40.9%  |
| 500+ KiB    | 5,706    | 38.5%  |

At 10 KiB, only 1.0% of programs are smaller than the minimum extension size,
and over 93% of programs are larger than 50 KiB — well above the proposed
minimum.

## Impact

The 10 KiB minimum makes griefing attacks cost approximately 0.072 SOL per
invocation, with all lamports irrecoverably donated to the victim's program
data account.

The minimum extension size has minimal impact on legitimate use:

- Programs needing less than 10 KiB of additional space must extend by the
  full 10 KiB minimum. The excess capacity is available for future use.
- Only 1.0% of mainnet programs have a total size below 10 KiB.
- No changes to the instruction's account list. No changes to signer
  requirements. No impact on existing tooling or multisig workflows.

## Security Considerations

The minimum extension size provides economic deterrence rather than absolute
prevention. An attacker willing to spend 0.072 SOL per slot can still trigger
program cache invalidation. However:

- The cost scales linearly with attack duration (~650 SOL/hour at 400ms slots).
- All lamports spent are donated to the victim, not burned.
- The attacker receives no benefit — the victim's program only gains additional
  allocated space.

This economic model makes sustained attacks prohibitively expensive while
preserving the permissionless nature of `ExtendProgram`.

## Backwards Compatibility

This feature places an additional constraint on an existing Loader V3
instruction (minimum extension size) and is therefore not fully backwards
compatible. Any caller currently extending by less than 10 KiB will need to
increase their extension amount. This change is gated behind a feature flag.
