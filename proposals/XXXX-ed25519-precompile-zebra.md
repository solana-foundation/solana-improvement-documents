---
simd: '453'
title: Ed25519 Precompile Verification Mode Flag
authors:
  - SK, ZZ
category: Standard
type: Core
status: Idea
created: 2026-01-27
feature: ed25519_precompile_verification_mode_flag
extends: '0152'
---

## Summary

Extend the Ed25519 signature verification precompile
(`Ed25519SigVerify111111111111111111111111111`) to support both `strict_verify`
(current behavior) and [ZIP-215](https://zips.z.cash/zip-0215) verification
semantics. The caller selects the verification mode via a flag byte, enabling
opt-in ZIP-215 verification while preserving backwards compatibility.

## Motivation

SIMD-0376 specifies the transition from `ed25519-dalek`'s `strict_verify` to
ZIP-215 verification for transaction signatures, gossip packets, and shred
packets. This SIMD addresses the Ed25519 precompile, but with a different
approach: rather than replacing `strict_verify`, this SIMD adds ZIP-215 as
an opt-in alternative while preserving the existing behavior.

### Why Not Simply Replace `strict_verify`?

The precompile has different security considerations than transaction
signatures:

1. **Cross-chain bridge security**: Applications may use the Ed25519 precompile
   to verify signatures from other chains (e.g., bridging from chains that use
   Ed25519). If those chains use stricter verification than ZIP-215, an
   attacker could craft signatures that pass ZIP-215 but would be rejected on
   the origin chain. This could enable forged bridge transactions.

2. **Small-order key exploits**: ZIP-215 accepts small-order public keys
   (torsion points) for which trivial signature forgeries exist (e.g.,
   $R = \mathcal{O}, S = 0$ verifies for any message). An attacker could
   deposit funds to such an address on another chain and then "bridge" them
   to Solana using forged signatures.

3. **Immutability expectations**: Existing applications may depend on the
   precompile's current behavior. Changing verification semantics could break
   security assumptions without the application's knowledge.

### The Solution: Caller-Selected Verification Mode

This SIMD repurposes the existing padding byte in the precompile instruction
format as a verification mode flag:

- `0x00`: Use `strict_verify` (current behavior, backwards compatible)
- `0x01`: Use ZIP-215 verification

This approach:

- **Preserves security**: Existing applications continue to use `strict_verify`
  without code changes.
- **Enables opt-in consistency**: New applications can choose ZIP-215 to match
  transaction signature verification semantics.
- **Maintains backwards compatibility**: The default mode (`0x00`) matches
  current behavior exactly.

## Scope Alignment with SIMD-0376

SIMD-0376 defines the transition from `strict_verify` to ZIP-215 across all
EdDSA verification contexts in Solana, listing the Ed25519 precompile as one
of its targets. This SIMD carves out the precompile-specific portion into a
separate specification with a different approach.

**This SIMD diverges from SIMD-0376's approach for the precompile.** Rather
than replacing `strict_verify` with ZIP-215, this SIMD adds ZIP-215 as an
opt-in mode while preserving `strict_verify` as the default. The precompile
is removed from SIMD-0376's activation scope; SIMD-0376's feature gate does
not affect the precompile.

The rationale for this different approach:

- **Bridge security**: Unlike transaction signatures, the precompile may be
  used to verify signatures from other chains with stricter verification
  rules. Replacing `strict_verify` could enable cross-chain exploits.

- **Backwards compatibility**: Existing applications expect `strict_verify`
  semantics. Changing the default could break security assumptions silently.

- **Flexibility**: Applications that want ZIP-215 consistency with transaction
  signatures can opt in explicitly.

## Dependencies

This proposal relates to the following SIMDs:

- **[SIMD-0152]: Precompiles** (Activated)

    Established the precompile specification and aligned the Ed25519 precompile
    with transaction signature verification using `strict_verify`.

- **[SIMD-0376]: Relaxing Transaction Signature Verification** (In Review)

    Defines the ZIP-215 verification algorithm and security rationale. This
    SIMD uses SIMD-0376's ZIP-215 specification but diverges from its approach
    by offering ZIP-215 as an opt-in mode rather than a replacement.

[SIMD-0152]: ./0152-precompiles.md
[SIMD-0376]: ./0376-relaxing-tx-sig-verification.md

## New Terminology

This proposal uses terminology defined in SIMD-0376:

- **Strict Verification**: Ed25519 verification that explicitly rejects both
  public keys and ephemeral points ($A$ and $R$) with torsion components or
  non-canonical encodings. Implemented by `ed25519-dalek`'s `verify_strict`.

- **Cofactored Verification**: A scheme where the verification equation is
  multiplied by the curve's cofactor (8), rendering torsion elements
  irrelevant while preserving security.

- **ZIP-215**: The EdDSA verification specification defined by
  [ZIP-215](https://zips.z.cash/zip-0215), used by Zcash, and implemented by
  `ed25519-zebra`.

## Detailed Design

Extend the Ed25519 precompile to support two verification modes, selected by
the caller via a flag byte.

### Verification Mode Flag

The existing padding byte in the precompile instruction format is repurposed
as a verification mode flag:

| Flag Value | Verification Mode | Behavior |
|------------|-------------------|----------|
| `0x00` | Strict | Use `strict_verify` (current behavior) |
| `0x01` | ZIP-215 | Use ZIP-215 cofactored verification |
| `0x02`-`0xFF` | Reserved | Return error |

Flag values `0x02` through `0xFF` are reserved for future use and MUST return
an error if specified.

### ZIP-215 Verification Algorithm (Flag = 0x01)

Given a 64-byte signature and 32-byte public key:

1. **Parse the signature**: Split the 64-byte signature into $R$ (first 32
   bytes) and $S$ (last 32 bytes, interpreted as a 32-byte little-endian
   integer).

2. **Validate $S$**: Reject the signature if $S \notin \{0, ..., l - 1\}$
   where $l$ is the order of the Ed25519 base point
   ($l = 2^{252} + 27742317777372353535851937790883648493$).

3. **Decode $R$**: Decode the first 32 bytes as a compressed Edwards point
   per ZIP-215 rules. Non-canonical encodings (where the y-coordinate
   $\geq p$) are accepted. Small-order points are accepted. Reject the
   signature if decompression fails per ZIP-215 rules.

4. **Decode $A$**: Decode the 32-byte public key as a compressed Edwards
   point per ZIP-215 rules. Non-canonical encodings (where the y-coordinate
   $\geq p$) are accepted. Small-order points are accepted. Reject the
   signature if decompression fails per ZIP-215 rules.

5. **Compute $h$**: Compute the hash $h = \text{SHA512}(R \| A \| M) \mod l$
   where $M$ is the message.

6. **Verify the equation**: Accept the signature if and only if:

$$8(S \cdot B) - 8R - 8(h \cdot A) = \mathcal{O}$$

where $B$ is the Ed25519 base point and $\mathcal{O}$ is the identity element.

### Strict Verification Algorithm (Flag = 0x00)

When the flag is `0x00`, the precompile uses the existing `strict_verify`
behavior as specified in SIMD-0152. This is unchanged from current behavior.

### Key Differences Between Modes

| Property | Strict (0x00) | ZIP-215 (0x01) |
|----------|---------------|----------------|
| Non-canonical $R$ encoding | Rejected | Accepted |
| Non-canonical $A$ encoding | Rejected | Accepted |
| Small-order $R$ | Rejected | Accepted |
| Small-order $A$ | Rejected | Accepted |
| $S$ canonicality ($S < l$) | Required | Required |
| Cofactor multiplication | No | Yes (×8) |

### Feature Gate

This SIMD requires a runtime feature gate:

```
ed25519_precompile_verification_mode_flag
```

This feature gate is separate from SIMD-0376's feature gate. When activated,
the Ed25519 precompile interprets the second byte as a verification mode flag,
enabling callers to select between `strict_verify` (`0x00`) and ZIP-215
(`0x01`) verification modes.

### Implementation

This SIMD only modifies the Ed25519 precompile
(`Ed25519SigVerify111111111111111111111111111`). Transaction signature
verification, gossip packet signatures, and shred packet signatures are
addressed by SIMD-0376's feature gate.

The precompile verifies each signature individually; batch verification is
not used.

The precompile MUST:

1. Parse the instruction data according to the precompile specification
   defined in SIMD-0152, with the padding byte now interpreted as a
   verification mode flag.

2. Validate the flag byte:
   - If `0x00`: use `strict_verify` mode.
   - If `0x01`: use ZIP-215 mode.
   - If `0x02`-`0xFF`: return an error.

3. For each signature to verify:
   - Extract the public key ($A$), signature ($R$, $S$), and message ($M$)
     using the offset-based data retrieval mechanism.
   - Verify the signature using the algorithm corresponding to the selected
     mode.
   - Return an error if verification fails.

4. Return success if all signatures verify successfully.

### Precompile Interface

The instruction format is modified from SIMD-0152 to repurpose the padding
byte:

```
struct PrecompileVerifyInstruction {
  num_signatures:    u8,
  verification_mode: u8,    
  offsets:           PrecompileOffsets[num_signatures],
  additionalData?:   Bytes,
}

struct PrecompileOffsets {
  signature_offset:            u16 LE,
  signature_instruction_index: u16 LE,
  public_key_offset:           u16 LE,
  public_key_instruction_index: u16 LE,
  message_offset:              u16 LE,
  message_length:              u16 LE,
  message_instruction_index:   u16 LE,
}
```

## Alternatives Considered

### Replace `strict_verify` with ZIP-215

Simply replace `strict_verify` with ZIP-215 in the precompile, matching
SIMD-0376's approach for transaction signatures. This was rejected because:

- **Bridge security risk**: Applications using the precompile to verify
  signatures from other chains could be exploited if those chains use stricter
  verification. An attacker could forge signatures that pass ZIP-215 but would
  be rejected on the origin chain.
- **Small-order key exploits**: ZIP-215 accepts small-order public keys
  (torsion points) for which trivial forgeries exist. An attacker could
  deposit funds to such addresses on other chains and bridge them to Solana
  using forged signatures.
- **Silent breakage**: Existing applications expect `strict_verify` semantics.
  Changing the default could break security assumptions without warning.

### Keep `strict_verify` Only

Maintain only `strict_verify` semantics for the precompile. This was rejected
because:

- Creates permanent inconsistency between transaction signatures and precompile
  verification after SIMD-0376 activates.
- Signatures valid for transaction authentication could be rejected by the
  precompile, which may confuse developers.
- Does not provide a path for applications that want ZIP-215 consistency.

### Add a New Precompile

Create a new precompile address for ZIP-215 verification, leaving the existing
precompile unchanged. This was rejected because:

- Adds unnecessary protocol complexity with two precompile addresses.
- The flag-based approach achieves the same flexibility with less overhead.
- Existing precompile address can accommodate both modes via the unused
  padding byte.

## Impact

This SIMD only affects the Ed25519 precompile. Transaction signature
verification, gossip packets, and shred packets are covered by SIMD-0376.

### Dapp Developers

- **No breaking changes**: Existing applications using `padding = 0x00`
  continue to work with `strict_verify` semantics.
- **Opt-in consistency**: Applications that want ZIP-215 verification (to
  match transaction signature semantics) can set the flag to `0x01`.
- **Migration path**: Applications can evaluate their security requirements
  and choose the appropriate verification mode.

### Validators

- **Dual implementation**: Validators must support both `strict_verify` and
  ZIP-215 verification modes.
- **Flag validation**: Validators must reject flag values outside `0x00`-`0x01`.

### Core Contributors

- **Library support**: Ensure both `ed25519-dalek` (for strict) and
  `ed25519-zebra` (for ZIP-215) verification paths are available.
- **Feature gate**: Implement and coordinate activation of a new feature gate
  separate from SIMD-0376.

## Security Considerations

### Cross-Chain Bridge Security

The primary security motivation for this design is protecting cross-chain
bridges. Applications using the precompile to verify signatures from other
chains must consider:

- **Verification mismatch attacks**: If the origin chain uses stricter
  verification than the precompile, an attacker could craft signatures that
  pass on Solana but would be rejected on the origin chain. This could enable
  forged bridge transactions.

- **Small-order key exploits**: ZIP-215 accepts small-order public keys
  (torsion points) for which trivial signature forgeries exist. An attacker
  could:
  1. Deposit funds to a small-order address on another chain.
  2. Forge signatures for that address that pass ZIP-215.
  3. Bridge funds to Solana.

**Recommendation**: Bridge applications SHOULD use strict mode (`0x00`) unless
they have verified that the origin chain's verification is compatible with
ZIP-215.

### EdDSA Security Properties

There are two important security properties for EdDSA schemes:

- **Strong Unforgeability under Chosen Message Attack (SUF-CMA)**: An attacker
  cannot create any new valid signature, even on a message that has been
  signed before.

- **Strongly Binding Signatures (SBS)**: Each valid signature corresponds to
  exactly one valid message.

Both verification modes achieve SUF-CMA by requiring $S \in \{0, ..., l - 1\}$.
Neither mode provides SBS, but SBS is not required for typical use cases.

### Mode Selection Guidance

| Use Case | Recommended Mode | Rationale |
|----------|------------------|-----------|
| Cross-chain bridges | Strict (`0x00`) | Matches stricter origin chain verification |
| Solana-native signatures | Either | Both modes accept canonically-encoded RFC 8032 signatures |
| Consistency with tx sigs | ZIP-215 (`0x01`) | Matches SIMD-0376 semantics |

Note: "Solana-native signatures" assumes keys and signatures are canonically
encoded per RFC 8032. Non-canonical encodings are only accepted in ZIP-215 mode.

For detailed security analysis of ZIP-215, see SIMD-0376.

## Backwards Compatibility

This change requires a dedicated feature gate, separate from SIMD-0376.

### Full Backwards Compatibility

This SIMD is fully backwards compatible:

- **Default behavior preserved**: Existing applications using `padding = 0x00`
  (which was the only valid value) continue to receive `strict_verify`
  semantics with no changes required.

- **No silent behavior changes**: Unlike a wholesale replacement of
  `strict_verify`, existing applications are not exposed to ZIP-215's broader
  acceptance criteria unless they explicitly opt in.

### Activation

Upon activation:

- Instructions with flag `0x00` behave identically to pre-activation.
- Instructions with flag `0x01` use ZIP-215 verification.
- Instructions with flag `0x02`-`0xFF` return an error.

**Note on padding byte behavior**: Per SIMD-0152, the current implementation
skips the padding byte without validation (see pseudocode: `data_position = 2`).
Standard SDKs set this byte to `0x00`. This SIMD assigns semantic meaning to
the byte, rejecting values `0x02`-`0xFF`. Applications using non-standard
padding values would be affected, though such usage is not expected in
practice.

### Migration Path

Applications do not need to migrate. The recommended approach:

1. **Existing applications**: No changes required. Continue using flag `0x00`
   for `strict_verify` behavior.

2. **New applications**: Evaluate security requirements:
   - Use `0x00` (strict) for cross-chain bridges or when matching other
     chains' verification.
   - Use `0x01` (ZIP-215) for consistency with Solana transaction signatures.

3. **Feature gate ordering**: This SIMD's feature gate can be activated
   independently of SIMD-0376.
