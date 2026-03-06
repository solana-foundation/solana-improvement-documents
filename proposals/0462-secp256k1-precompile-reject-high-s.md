---
simd: '0462'
title: secp256k1 Precompile - Reject High-S Signatures
authors:
  - SK, ZZ
category: Standard
type: Core
status: Idea
created: 2026-03-06
feature: HgNAvKS5GrJbV57y1r8rF1FPmMet6WxEvSnChEeuy4QY
---

## Summary

Switch the secp256k1 precompile's public-key recovery backend from
`libsecp256k1` to `k256` (via `solana-secp256k1-recover`). The only
externally visible behavior change is that malleable (high-S) signatures,
which were previously accepted by `libsecp256k1`, are now rejected with
`PrecompileError::InvalidSignature`.

## Motivation

The `libsecp256k1` crate is no longer the preferred maintenance path for
Solana ecosystem crates. `k256` is actively maintained and is already used
by `solana-secp256k1-recover`, the canonical recovery primitive elsewhere in
the runtime. Consolidating on a single, well-audited backend reduces
maintenance burden and long-term dependency risk.

As a side effect of this migration, the precompile gains stricter signature
malleability enforcement. The secp256k1 curve allows two distinct (signature,
recovery-id) pairs to recover the same public key: one with a low-S value
and one with a high-S value. Accepting both creates transaction malleability
surface that smart contracts may rely on unexpectedly. The `k256` backend
requires canonical low-S form and rejects high-S signatures outright, which
is consistent with the behavior of Bitcoin and most modern secp256k1
implementations (BIP-62, EIP-2).

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119.

## New Terminology

- **High-S signature**: A secp256k1 ECDSA signature whose `s` scalar is
  greater than `n/2`, where `n` is the curve order. Such signatures are the
  malleable complement of a corresponding low-S signature and recover to the
  same public key under a flipped recovery ID.

- **Low-S signature**: A secp256k1 ECDSA signature whose `s` scalar is at
  most `n/2`. This is the canonical form required by BIP-62 and EIP-2.

## Detailed Design

### Current behavior

The secp256k1 precompile (`NativeLoader` program address
`KeccakSecp256k11111111111111111111111111111`) performs the following steps
for each signature slot encoded in the instruction data:

1. Slice the 64-byte compact signature and 1-byte recovery ID from the
   referenced instruction.
2. Slice the message bytes and compute `Keccak256(message)`.
3. Call `libsecp256k1::recover(message_hash, signature, recovery_id)`.
4. Derive the Ethereum address from the recovered uncompressed public key.
5. Compare the derived address with the expected address encoded in the
   instruction.

`libsecp256k1` accepts both low-S and high-S signatures. A caller can
construct two distinct instruction payloads that each pass verification for
the same signer: one using the canonical low-S signature with recovery ID
`r`, and one using the high-S complement with recovery ID `r ^ 1`.

### Proposed behavior

When the feature gate `secp256k1_precompile_use_k256`
(`HgNAvKS5GrJbV57y1r8rF1FPmMet6WxEvSnChEeuy4QY`) is active, step 3 above
MUST use `solana_secp256k1_recover::secp256k1_recover` (backed by `k256`)
instead of `libsecp256k1`.

The `k256` backend enforces low-S normalization. A high-S signature MUST be
rejected with `PrecompileError::InvalidSignature`. All other behaviors remain
identical:

| Input condition | Before activation | After activation |
|---|---|---|
| Valid low-S signature | `Ok(())` | `Ok(())` |
| Valid high-S signature | `Ok(())` | `Err(InvalidSignature)` |
| Recovery ID >= 4 | `Err(InvalidRecoveryId)` | `Err(InvalidRecoveryId)` |
| Malformed signature bytes | `Err(InvalidSignature)` | `Err(InvalidSignature)` |
| Invalid public key match | `Err(InvalidSignature)` | `Err(InvalidSignature)` |

### Feature gate

The behavioral change MUST be gated on feature
`secp256k1_precompile_use_k256`
(`HgNAvKS5GrJbV57y1r8rF1FPmMet6WxEvSnChEeuy4QY`). Before activation, the
existing `libsecp256k1` backend MUST remain in use and high-S signatures
MUST continue to be accepted. After activation, the `k256` backend MUST be
used exclusively.

### No change to other precompile behavior

The `Ed25519` precompile and the `secp256r1` precompile (SIMD-0075) are not
affected by this change.

## Alternatives Considered

### Do not enforce low-S at the precompile level

Malleability enforcement could be left to individual on-chain programs.
This is rejected because it makes cross-program and cross-client reasoning
harder: the "same signer" can produce two distinct, accepted precompile
instructions, creating a footgun for programs that use the precompile as an
authentication check.

### Enforce low-S without switching the backend

Low-S enforcement could be added as an explicit check on top of `libsecp256k1`
recovery. This would achieve the same observable behavior but still carries
the maintenance cost of depending on `libsecp256k1`. Switching backends is
preferred.

### Switch backend without a feature gate (immediate activation)

Because high-S signatures are currently accepted on all clusters, removing
acceptance without a feature gate would be a non-deterministic consensus
break for any in-flight transaction relying on high-S acceptance. A feature
gate is required.

## Impact

### For dApp developers / wallets

- Wallets and signing libraries that produce canonicalized (low-S) signatures
  (the standard for all widely-used secp256k1 implementations) are unaffected.
- Programs or off-chain code that deliberately construct or relay high-S
  malleable signatures will observe `PrecompileError::InvalidSignature` after
  activation. Such usage is non-standard; low-S equivalents can be substituted.

### For validators / core contributors

- All validator clients MUST implement identical low-S rejection semantics
  after the feature gate activates to preserve consensus.
- The `libsecp256k1` dependency in the precompile path can be removed in a
  follow-up cleanup after full network activation.

## Security Considerations

### Malleability reduction

Accepting high-S signatures allows an observer to transform any accepted
secp256k1 instruction into a second accepted instruction with a different
transaction ID but the same semantic authorization. This is a known
malleability vector (analogous to the Bitcoin pre-BIP-62 malleability issue).
After activation, each valid secp256k1 authorization has a unique
canonical form, reducing the attack surface for replay or substitution
attacks.

### Cross-client consistency

Any difference in low-S enforcement logic between validator clients would
cause a consensus split. All implementations MUST use the same threshold
(`s <= n/2`) for rejection, where `n` is the secp256k1 curve order
`0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141`.
Implementations MUST validate correctness using the behavior parity tests
in the agave reference implementation, covering: valid low-S signatures,
high-S malleable signatures (must be rejected), and recovery IDs outside
[0, 3] (must return `InvalidRecoveryId`).

## Backwards Compatibility

Transactions using only low-S secp256k1 signatures are unaffected. Transactions
using high-S signatures will fail after the feature gate activates. Because
high-S usage is non-standard and no legitimate signer should produce such
signatures, the practical impact is expected to be negligible. The feature
gate provides a safe staged rollout and observation window before broad
activation.

## References

- [BIP-62: Dealing with malleability](https://github.com/bitcoin/bips/blob/master/bip-0062.mediawiki)
- [EIP-2: Homestead Hard-fork Changes](https://eips.ethereum.org/EIPS/eip-2)
- [SIMD-0075: Precompile for secp256r1 sigverify](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0075-precompile-for-secp256r1-sigverify.md)
- [agave implementation](https://github.com/anza-xyz/agave/)
