---
simd: '0376'
title: Relaxing Transaction Signature Verification
authors:
  - David Rubin (Syndica)
category: Standard
type: Core
status: Review
created: 2025-10-06
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal replaces the `verify_strict` semantics used in Solana, derived
from Agave's usage of `ed25519-dalek`'s `verify_strict`, with
[ZIP-215](https://zips.z.cash/zip-0215), Zebra's EdDSA variant. In practice,
this removes the $R$ and $A$ torsion checks, multiplies the verification
equation by the cofactor, and makes verification insensitive to torsion
elements. This change enables batch verification of transaction signatures,
improves validator efficiency, and standardizes consensus behaviour across
implementations.

## Motivation

Two main factors motivate this proposed change.

1. Today, Solana validators must perfectly replicate the behaviour of
`verify_strict`, a function implemented in the `ed25519-dalek` library.
This forces validators written in other languages, or ones that do not
wish to use this library for better diversity, to re-implement the exact
semantics. While this will always be the case, due to transaction verification
being part of consensus, this proposal suggests using a better proven
verification equation, which is better described,
rather than the implicit behaviour found in the `ed25519-dalek` library.

2. `verify_strict` rejects small-order `R`, which makes batch verification
impossible. Batched verification can reduce costs by ~40% for large
signatures batches, which is significant at Solana's scale, where validators
process hundreds of thousands of signatures every block.

## New Terminology

- Strict Verification: Ed25519 verification that explicitly rejects both public
keys and ephemeral points in the signature ($A$ and $R$) with torsion
components.
- Cofactored Verification: A scheme where the entire verification equation
is multiplied by the curve's cofactor (8), rendering torsion elements irrelevant
while preserving security.
- Batch Verification: Verifying many signatures together via random linear
combination and a multi-scalar multiplication.

## Detailed Design

Switch to using the verification equation described in
[ZIP-215](https://zips.z.cash/zip-0215) for Ed25519 EdDSA signature
verification.

### Algorithm:

Given a message $M$, public key $A$, signature ephemeral point $R$, and
scalar $S$.

1. Reject the signature if $`S \notin \{0, ..., L - 1\}`$.
2. Compute the hash $\text{SHA512}(R \|\| A \|\| M)$ and reduce it
$\bmod L$ to get scalar $h$.
3. Given that $B$ is the Ed25519 basepoint, accept the signature if:

```math
8(S \cdot B) - 8R - 8(h \cdot A) = \mathcal{O}
```

Note that non-canonical $R$ and $A$ points *are allowed*. Honest parties
will generate their keys according to the protocol, which in this case
would be [RFC-8032](https://www.rfc-editor.org/rfc/rfc8032.html#section-5.1.6)'s
definition of `sign`. As this does not produce non-canonical encodings of
points, the honest parties will be unaffected, and it will only affect parties
that purposefully create special signatures.

### Application:

This proposal specifically targets usages of `verify_strict`, replacing
them with the Algorithm described above. This includes replacing the equation
used for verification of transaction signatures, gossip packet signatures, shred
packet signatures, and the Ed25519 precompile program.

Section 3.2 of [Taming the many EdDSAs](https://eprint.iacr.org/2020/1244.pdf)
explains the relationship between batched and single cofactored verifications,
proving them to be compatible. As a result, they can be used interchangeably,
in use cases such as optimizing transaction signature verification.

## Alternatives Considered

- `ed25519-dalek`'s `verify`: Another option would be to just downgrade the
check from `verify_strict` to `verify`. This would also be backwards compatible,
however there are a few issues with this approach. It is not possible to
perform a compatible batched verification of a cofactorless verification
equation with some sort of incompatibility, leading back to the original issue.
Our only option would be to define the protocol in terms of the batched
verification equation's behaviour which is not preferable.

- *Taming the many EdDSAs* equation: The paper describes a cofactored
verification scheme very similar to ZIP-215, the only difference being
that small-order $A$ points are rejected. This allows their scheme to achieve
strongly binding signatures, a property that does not affect Solana. We prefer
using ZIP-215 as it has a well-proven Rust library,
[ed25519-zebra](https://github.com/ZcashFoundation/ed25519-zebra),
that would allow easier migration for Agave.

## Impact

- Dapp developers: No required changes, signatures already generated remain
valid.
- Validators: Lower CPU usage, faster verification pipelines.
- Core Contributors: A more clear, standardized implementation for new validator
clients and other software potentially performing transaction

## Security Considerations

There are two important qualities an EdDSA scheme can have.

- Strongly Binding Signatures (SBS): A signature scheme is *strongly binding*
if each valid signature corresponds to exactly one valid message, i.e there
is no "malleability" in the verification equation.
- Strong Unforgeability under Chosen Message Attack (SUF-CMA): A signature
scheme is SUF-CMA secure if an attacker cannot create *any* new valid signature,
even on a message that has been signed before. In other words, they can't
"malleate" an existing signature into another distinct, valid one.

The only quality that Solana worries about is SUF-CMA (as opposed to EUF-CMA),
which ZIP-215 achieves by rejecting $S$ scalars which do not fit into $l$.

## Backwards Compatibility

- All signatures valid under `verify_strict` remain valid under ZIP-215
verification.
- A small class of signatures previously rejected may now be accepted.

This upgrade will require one feature gate. Once this feature gate is active,
ZIP-215 will be the equation used for all EdDSA signature verifications,
instead of `verify_strict`. 

Here is a proof that any signature that `verify_strict` accepts would
be accepted by the new verification equation as well:

### Lemma:

Consider the `verify_strict` equation (E) to be:

```math
S \cdot B - h \cdot A = R
```

where $B$ is the base point, $A$ is the public key point,
$R$ is the ephemeral point, $S$ is the signature scalar and
$h = \text{SHA512}(R \|\| A \|\| M) \bmod L$.

Assume `verify_strict` accepts a signature, where

1. The above equation (E) holds in the Edwards25519 group.
2. $S$ is canonical and $`S \in \{0, ..., L - 1\}`$.
3. $A$ is canonical and *not* a small-order point.
4. $R$ is canonical and *not* a small-order point.

Then the new verification equation (C):

```math
8(S \cdot B) - 8R - 8(h \cdot A) = \mathcal{O}
```

also holds; therefore a new verifier that enforces (2), and the equation (C),
will accept the signature.

### Proof:

Start from (E):

```math
S \cdot B - h \cdot A = R
```

This can be rewritten as:

```math
S \cdot B - R - h \cdot A = \mathcal{O}
```

Apply scalar multiplication by the cofactor (8). Since scalar multiplication
is linear and the group law applies:

```math
[8](S \cdot B - R - h \cdot A) = [8]\mathcal{O}
```

If you distribute $[8]$ across the sum:

```math
8(S \cdot B) - 8R - 8(h \cdot A) = \mathcal{O}
```

which is exactly the equation (C). Therefore, assuming that $S$ is properly
checked, the new verification equation should never reject a signature accepted
by `verify_strict`.
