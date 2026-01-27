---
simd: 'XXXX'
title: Ed25519 Precompile Signature Verification with ZIP-215
authors:
  - SK, ZZ
category: Standard
type: Core
status: Idea
created: 2026-01-27
feature: ed25519_precompile_verify_strict_to_zip215
extends: '0152'
---

## Summary

Update the Ed25519 signature verification precompile
(`Ed25519SigVerify111111111111111111111111111`) to use
[ZIP-215](https://zips.z.cash/zip-0215) verification semantics via
`ed25519-zebra`, replacing the current `strict_verify` implementation from
`ed25519-dalek`.

## Motivation

SIMD-0376 specifies the transition from `ed25519-dalek`'s `strict_verify` to
ZIP-215 verification across Solana's EdDSA usage, including transaction
signatures, gossip packets, shred packets, and the Ed25519 precompile. This
SIMD provides the concrete specification and feature gate for the Ed25519
precompile portion of that transition.

Separating the precompile change into its own SIMD allows:

1. **Independent rollout**: The precompile can be activated separately from
   transaction signature changes, enabling staged deployment and validation.

2. **Explicit specification**: Precompile behavior is consensus-critical and
   warrants explicit documentation of decoding rules and acceptance criteria.

3. **Feature gate isolation**: A dedicated feature gate allows the precompile
   change to be tested and activated independently.

The benefits of ZIP-215 for the precompile:

- **Standardization**: ZIP-215 provides a well-specified verification
  equation, simplifying alternative validator implementations.

- **Consistency**: After this SIMD activates, the precompile uses the same
  verification semantics as transaction signatures.

Note: ZIP-215 allows batch verification in general, but this SIMD does not
require or target it for the precompile. The primary goals are standardization
and consistency with transaction signature verification.

## Scope Alignment with SIMD-0376

SIMD-0376 defines the transition from `strict_verify` to ZIP-215 across all
EdDSA verification contexts in Solana, listing the Ed25519 precompile as one
of its targets. This SIMD carves out the precompile-specific portion of that
transition into a separate specification with its own feature gate.

**This SIMD refines SIMD-0376's scope by providing the concrete specification
for the Ed25519 precompile.** The precompile is removed from SIMD-0376's
activation scope; the precompile change is governed solely by this SIMD's
feature gate. SIMD-0376's feature gate does not affect the precompile. There
is no double-activation requirement.

The rationale for a separate SIMD and feature gate:

- **Staged activation**: Transaction signature verification (the primary
  SIMD-0376 target) can be activated and validated on mainnet before the
  precompile changes.

- **Precompile-specific concerns**: Precompiles have unique considerations
  around instruction data parsing and error handling that warrant explicit
  specification.

- **Rollback isolation**: If issues arise with the precompile change, it can
  be disabled independently without affecting transaction verification.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0152]: Precompiles**

    Established the precompile specification and aligned the Ed25519 precompile
    with transaction signature verification using `strict_verify`.

- **[SIMD-0376]: Relaxing Transaction Signature Verification**

    Defines the ZIP-215 verification algorithm and security rationale. This
    SIMD implements the precompile portion of SIMD-0376's scope.

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

Replace the signature verification function in the Ed25519 precompile with
ZIP-215 verification semantics, as implemented by `ed25519-zebra`.

### Verification Algorithm

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

### Key Differences from `strict_verify`

| Property | `strict_verify` | ZIP-215 |
|----------|-----------------|---------|
| Non-canonical $R$ encoding | Rejected | Accepted |
| Non-canonical $A$ encoding | Rejected | Accepted |
| Small-order $R$ | Rejected | Accepted |
| Small-order $A$ | Rejected | Accepted |
| $S$ canonicality ($S < l$) | Required | Required |
| Cofactor multiplication | No | Yes (×8) |

### Feature Gate

This SIMD requires a runtime feature gate:

```
ed25519_precompile_verify_strict_to_zip215
```

This feature gate is separate from SIMD-0376's feature gate. When activated,
the Ed25519 precompile switches from `strict_verify` to ZIP-215 verification.

### Implementation

This SIMD only modifies the Ed25519 precompile
(`Ed25519SigVerify111111111111111111111111111`). Transaction signature
verification, gossip packet signatures, and shred packet signatures are
addressed by SIMD-0376's feature gate.

The precompile verifies each signature individually; batch verification is
not used.

The precompile MUST:

1. Parse the instruction data according to the existing precompile
   specification defined in SIMD-0152.

2. For each signature to verify:
   - Extract the public key ($A$), signature ($R$, $S$), and message ($M$)
     using the offset-based data retrieval mechanism.
   - Verify the signature using the ZIP-215 algorithm specified above.
   - Return an error if verification fails.

3. Return success if all signatures verify successfully.

### Precompile Interface

The instruction format remains unchanged from SIMD-0152:

```
struct PrecompileVerifyInstruction {
  num_signatures:  u8,
  padding:         u8,
  offsets:         PrecompileOffsets[num_signatures],
  additionalData?: Bytes,
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

### Keep `strict_verify` for Precompile

Maintain the current `strict_verify` semantics for the precompile while
transactions use ZIP-215. This was rejected because:

- Creates inconsistency between transaction signatures and precompile
  verification.
- Signatures valid for transaction authentication could be rejected by the
  precompile.

### Single Feature Gate with SIMD-0376

Use SIMD-0376's feature gate to activate all EdDSA verification changes
atomically, including the precompile. This was rejected because:

- Precompile changes affect on-chain program behavior and warrant independent
  testing.
- Transaction signature verification is more critical; it should stabilize
  before the precompile changes.
- A precompile-specific issue should not require rolling back transaction
  verification changes.

This SIMD introduces a separate feature gate for the precompile. The two
feature gates (SIMD-0376 for transactions/gossip/shreds, this SIMD for the
precompile) can be activated in either order, though activating SIMD-0376
first is recommended.

## Impact

This SIMD only affects the Ed25519 precompile. Transaction signature
verification, gossip packets, and shred packets are covered by SIMD-0376.

### Dapp Developers

- **No breaking changes**: All signatures currently valid under `strict_verify`
  remain valid under ZIP-215.
- **Consistency**: After both SIMD-0376 and this SIMD activate, signatures
  that work for transaction authentication also work with the precompile.

### Validators

- **Simplified implementation**: ZIP-215 is a well-documented specification,
  making the precompile easier to implement in alternative validator clients.

### Core Contributors

- **Library update**: Replace `ed25519-dalek` with `ed25519-zebra` for the
  precompile verification path.
- **Feature gate**: Implement and coordinate activation of a new feature gate
  separate from SIMD-0376.

## Security Considerations

There are two important security properties for EdDSA schemes:

- **Strong Unforgeability under Chosen Message Attack (SUF-CMA)**: An attacker
  cannot create any new valid signature, even on a message that has been
  signed before. This prevents signature malleation.

- **Strongly Binding Signatures (SBS)**: Each valid signature corresponds to
  exactly one valid message, with no ambiguity in the verification equation.

ZIP-215 achieves SUF-CMA by requiring $S \in \{0, ..., l - 1\}$. This is the
security property relevant to Solana's use cases. ZIP-215 does not provide
SBS, but SBS is not required for the precompile's security model.

The relaxation of $R$ and $A$ checks does not affect security for honest
users:

- Keys and signatures generated per RFC 8032 do not contain non-canonical
  encodings or torsion components.
- Only adversarially crafted inputs could contain these elements, and such
  signatures cannot be forged for keys the attacker does not control.
- Small-order public keys are inherently attacker-controlled; honest key
  generation cannot produce them.

The precompile continues to verify signatures over user-specified messages,
providing the same semantic guarantees as before activation.

For detailed security analysis, including the SUF-CMA proof, see SIMD-0376.

## Backwards Compatibility

This change requires a dedicated feature gate, separate from SIMD-0376.

Upon activation:

- **All signatures valid under `strict_verify` remain valid**: The cofactored
  equation accepts a strict superset of signatures accepted by `strict_verify`.
  SIMD-0376 provides a formal proof that any signature passing `strict_verify`
  also passes ZIP-215 verification.

- **Some previously rejected signatures become valid**: Signatures with
  non-canonical $R$ or $A$ encodings, or small-order $R$ or $A$ points, that
  were previously rejected will now be accepted. These signatures cannot be
  produced by standard libraries following RFC 8032 and do not affect honest
  users.

The recommended upgrade path:

1. Activate SIMD-0376 feature gate for transaction signatures, gossip packets,
   and shred packets.
2. Activate this SIMD's feature gate for the Ed25519 precompile.

This ordering allows transaction signature verification to stabilize before
modifying precompile behavior. However, the feature gates are independent and
can be activated in either order without correctness issues.
