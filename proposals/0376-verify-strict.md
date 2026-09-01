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
from Agave's usage of `ed25519-dalek`'s `verify_strict`, with the *cofactored*
verification scheme described in
[Taming the many EdDSAs](https://eprint.iacr.org/2020/1244.pdf):

- the cofactored verification equation of
[ZIP-215](https://zips.z.cash/zip-0215),
- combined with explicit rejection of non-canonical encodings and of
small-order $A$ and $R$.

In practice, this multiplies the verification equation by the cofactor, making
verification insensitive to the torsion *components* of otherwise valid points,
while continuing to reject points that are *entirely* torsion. This change
enables batch verification of transaction signatures, improves validator
efficiency, and standardizes consensus behaviour across implementations.

Note that this proposal deliberately does *not* adopt ZIP-215 verbatim.
ZIP-215 accepts small-order $A$, which on Solana would make
`Pubkey::default()` (the all-zero encoding, also the System Program ID) a
signable public key. See [Security Considerations](#security-considerations).

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

2. `verify_strict` uses the *cofactorless* verification equation
$`S \cdot B - h \cdot A = R`$. Batch verification is inherently cofactored: it
checks a random linear combination of the individual equations multiplied by the
cofactor, and therefore cannot reproduce cofactorless accept/reject decisions.
It is the cofactorless equation, not the point checks, that makes batch
verification of `verify_strict` semantics impossible. Batched verification can
reduce costs by ~40% for large signature batches, which is significant at
Solana's scale, where validators process hundreds of thousands of signatures
every block.

The per-signature checks retained by this proposal (canonicity, scalar
reduction, small-order rejection) are compatible with batching: they are
independent of the batch and are applied to each signature before it enters
the multi-scalar multiplication.

## New Terminology

- Strict Verification: Ed25519 verification that explicitly rejects both public
keys and ephemeral points in the signature ($A$ and $R$) with torsion
components.
- Cofactored Verification: A scheme where the entire verification equation
is multiplied by the curve's cofactor (8), rendering torsion elements irrelevant
while preserving security.
- Batch Verification: Verifying many signatures together via random linear
combination and a multi-scalar multiplication.
- Canonical Point Encoding: A 32-byte compressed Edwards encoding in which the
low 255 bits encode a $y$ coordinate that is fully reduced ($y < 2^{255} - 19$),
and in which the sign bit (bit 255) is not set when the recovered
$x$ coordinate is zero.
- Small-Order Point: A point $P$ in the order-8 torsion subgroup of
Edwards25519, i.e. one satisfying $`[8]P = \mathcal{O}`$. There are exactly
eight such points, including the identity.

## Detailed Design

Switch to using the cofactored verification equation described in
[ZIP-215](https://zips.z.cash/zip-0215) for Ed25519 EdDSA signature
verification, while making the encoding and small-order checks explicit.

### Algorithm:

Given a message $M$, a 32-byte public key encoding $A$, and a 64-byte
signature split into a 32-byte ephemeral point encoding $R$ and a 32-byte
scalar encoding $S$.

1. Reject the signature if the encoding of $A$ is not canonical.
2. Reject the signature if the encoding of $R$ is not canonical.
3. Reject the signature if $S$ is not fully reduced, i.e. if
$`S \notin \{0, ..., L - 1\}`$.
4. Decode $A$ and $R$ to curve points. Reject the signature if either point
is not on the curve.
5. Reject the signature if $A$ is small-order, i.e. if
$`[8]A = \mathcal{O}`$. Reject the signature if $R$ is small-order.
6. Compute the hash $\text{SHA512}(R \|\| A \|\| M)$ and reduce it
$\bmod L$ to get scalar $h$.
7. Given that $B$ is the Ed25519 basepoint, accept the signature if:

```math
8(S \cdot B) - 8R - 8(h \cdot A) = \mathcal{O}
```

Steps 1–5 define the input-validation policy for this proposal; they are not all
inherited from a single existing verification rule. RFC-8032 requires canonical
point decoding and a fully reduced $S$, but does not separately reject
small-order $A$ or $R$. Existing `verify_strict` paths reject a non-reduced $S$
and small-order $A$ and $R$, while their treatment of non-canonical point
encodings depends on the `ed25519-dalek` version and call path.

Step 7 is the principal relaxation: multiplying the verification equation by
the cofactor makes verification insensitive to a torsion component *added* to
an otherwise valid $A$ or $R$. This permits compatible batched verification,
while step 5 continues to reject points that consist of nothing but torsion.

Because steps 1 and 2 have already rejected non-canonical encodings, step 5 may
be implemented either as a cofactor multiplication or as a comparison of the
32-byte encodings of $A$ and $R$ against the eight canonical small-order
encodings listed in [Test Vectors](#test-vectors).

Honest parties will generate their keys and signatures according to
[RFC-8032](https://www.rfc-editor.org/rfc/rfc8032.html#section-5.1.6)'s
definition of `sign`. This produces neither non-canonical encodings nor
small-order points, so honest parties are unaffected by steps 1–5, and only
parties that purposefully create special keys or signatures can be affected.

### Application:

This proposal specifically targets usages of `verify_strict`, replacing
them with the Algorithm described above. This includes replacing the equation
used for verification of transaction signatures, gossip packet signatures, shred
packet signatures, and the Ed25519 precompile program.

Section 3.2 of [Taming the many EdDSAs](https://eprint.iacr.org/2020/1244.pdf)
explains the relationship between batched and single cofactored verifications,
proving them to be compatible. As a result, they can be used interchangeably,
in use cases such as optimizing transaction signature verification.
The batched verification implementation in
[ed25519-zebra](https://github.com/ZcashFoundation/ed25519-zebra) can still be
used, provided steps 1–5 are applied to each signature before it is added to
the batch, since those steps are per-signature and independent of the batch.

## Alternatives Considered

- `ed25519-dalek`'s `verify`: Another option would be to just downgrade the
check from `verify_strict` to `verify`. This would also be backwards compatible,
however there are a few issues with this approach. It is not possible to
perform a compatible batched verification of a cofactorless verification
equation with some sort of incompatibility, leading back to the original issue.
Our only option would be to define the protocol in terms of the batched
verification equation's behaviour which is not preferable.

- Unmodified ZIP-215: Earlier revisions of this proposal adopted ZIP-215
verbatim, which additionally accepts non-canonical encodings and small-order
$A$ and $R$. This was rejected. ZIP-215's containment argument is that a
forgeable weak key can only compromise an authority that was never safe in the
first place, which holds in Zcash, where an Ed25519 public key is only ever a
verification key. It does not hold on Solana, where the same 32 bytes are also
an address, a program ID, and an authority sentinel. Accepting small-order $A$
would make `Pubkey::default()` signable; see
[Security Considerations](#security-considerations). Rejecting small-order $A$
and $R$ costs at most a byte-string comparison per signature and preserves the
batching design, so the cost of retaining these checks is negligible.

- Rejecting only the all-zero public key: Rejecting only the all-zero encoding
would address the known sentinel value, but Edwards25519 has eight canonical
small-order encodings (and six further non-canonical ones), any of which an
application could use as a constant. Rejecting the whole torsion subgroup is
no more expensive and is not value-specific.

## Impact

- Dapp developers: No required changes for signatures produced by RFC-8032
`sign`; those remain valid.
- Validators: Lower CPU usage, faster verification pipelines.
- Core Contributors: A more clear, standardized implementation for new validator
clients and other software potentially performing signature
verification.

## Security Considerations

There are two important qualities an EdDSA scheme can have.

- Strongly Binding Signatures (SBS): A signature scheme is *strongly binding*
if each valid signature corresponds to exactly one valid message, i.e there
is no "malleability" in the verification equation.
- Strong Unforgeability under Chosen Message Attack (SUF-CMA): A signature
scheme is SUF-CMA secure if an attacker cannot create *any* new valid signature,
even on a message that has been signed before. In other words, they can't
"malleate" an existing signature into another distinct, valid one.

The scheme in this proposal achieves SUF-CMA by rejecting $S$ scalars that are
not fully reduced $\bmod\ L$ (step 3), and achieves SBS by rejecting
small-order $A$ (step 5), matching the cofactored scheme analyzed in
*Taming the many EdDSAs*.

### Rejecting small-order public keys

The cofactored equation annihilates every small-order component, so if
small-order $A$ were accepted, a small-order public key would accept a forged
signature on *any* message. Concretely, the all-zero 32-byte encoding is a
canonical encoding of a point of order 4. Setting $A$ and $R$ to that encoding
and $S = 0$ makes every term of the cofactored equation the identity
independently of $h$, so the all-zero 64-byte signature would verify for the
all-zero public key on every message. `verify_strict` rejects this witness
today, and step 5 of this proposal continues to reject it.

This matters on Solana specifically because public keys are not only
verification keys. The all-zero encoding is `Pubkey::default()`, is rendered as
`11111111111111111111111111111111`, and is the System Program ID. 

Rejecting the torsion subgroup at the verifier removes this class of
cross-layer collision for all eight canonical small-order encodings, not just
the all-zero one, and does so without weakening the batching design that
motivates the proposal.

### Test Vectors

Implementations must reject each of the following as $A$ and as $R$. These are
the eight canonical encodings of the order-8 torsion subgroup, and must be
rejected by step 5:

```text
0000000000000000000000000000000000000000000000000000000000000000  (order 4)
0000000000000000000000000000000000000000000000000000000000000080  (order 4)
0100000000000000000000000000000000000000000000000000000000000000  (order 1)
26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05  (order 8)
26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85  (order 8)
c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a  (order 8)
c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa  (order 8)
ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f  (order 2)
```

The following six encodings are the non-canonical encodings of small-order
points, and must be rejected by steps 1 and 2. The first four have
$y \ge 2^{255} - 19$; the last two set the sign bit while the recovered $x$
coordinate is zero:

```text
edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f
edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f
eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
0100000000000000000000000000000000000000000000000000000000000080
ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
```

In particular, the all-zero public key paired with the all-zero 64-byte
signature must be rejected for every message.

These fourteen encodings are exactly the set enumerated by `ed25519-zebra`'s
`small_order` test suite: the eight above are the compressed encodings of
`curve25519-dalek`'s `EIGHT_TORSION`, and the six above are the low-order
prefix of that suite's non-canonical encodings. That suite constructs all
14x14 combinations of $A$ and $R$ drawn from this set with $S = 0$, and
asserts that every one of them is *accepted* under unmodified ZIP-215, by both
its individual and its batched verifier. `verify_strict` rejects all of them
today, and a conforming implementation of this proposal must continue to reject
all of them; rejecting each encoding in either position covers all 196
combinations.

## Backwards Compatibility

This upgrade will require one feature gate. Once this feature gate is active,
the equation and checks described above will be used for all EdDSA signature
verifications, instead of `verify_strict`.

- All signatures accepted by `verify_strict` with canonical $A$ and $R$
encodings remain valid.
- A small class of signatures previously rejected may now be accepted: those
where $A$ or $R$ carries a torsion component but is not itself small-order.
- Some verification paths may reject signatures that they previously accepted
when $A$ or $R$ has a non-canonical encoding. This impact is path-dependent:
`ed25519-dalek` 1.x decodes $A$ and $R$ permissively and compares curve points,
whereas 2.x rejects a non-canonical $R$ by comparing the recomputed canonical
encoding of $R$ with the signature bytes, while still decoding $A$
permissively. Such encodings cannot be produced by RFC-8032 `sign` and can only
arise from purpose-built keys or signatures. Implementations must therefore
evaluate the existing acceptance set at each affected call site rather than
assuming that all `verify_strict` paths behave identically.

Explicit canonicity checks are preferred over inheriting version-dependent
decoding behaviour, since reproducing implicit library behaviour across
validator implementations is precisely the problem this proposal sets out to
remove. A feature gate is required in every case because the cofactored
equation also accepts signatures that the existing equation rejects, regardless
of whether canonicity checks narrow a particular path's accepted set.

Here is a proof that any signature that `verify_strict` accepts, and whose $A$
and $R$ encodings are canonical, would be accepted by the new verification
equation as well:

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

also holds; therefore a new verifier that enforces (2), (3), (4) and the
equation (C), will accept the signature.

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

which is exactly the equation (C). Therefore, assuming that $S$, $A$ and $R$
are properly checked, the new verification equation should never reject a
signature accepted by `verify_strict` under the assumptions above.
