---
simd: "0075"
title: Precompile for verifying secp256r1 sig.
authors:
  - Orion (Bunkr)
  - Jstnw (Bunkr)
  - Dean (Web3 Builders Alliance)
category: Standard
type: Core
status: Draft
created: 2024-02-27
feature: (fill in with feature tracking issues once accepted)
supersedes: "0048"
---

## Summary

Adding a precompile to support the verification of signatures generated on
the secp256r1 curve. Analogous to the support for secp256k1 and ed25519
signatures that already exists in form of the
`KeccakSecp256k11111111111111111111111111111` and
`Ed25519SigVerify111111111111111111111111111` precompiles.

## Motivation

Solana has the opportunity to leverage the secure element of users' existing
mobile devices to support more user-friendly self-custodial security solutions.
The status quo of air-gapping signing with a hardware wallet currently requires
specialty hardware and still represents a single point of failure. Multi-signature
wallets provide enhanced security through multi-party signing, however the UX
is cumbersome due to the need to sign transactions multiple times and manage
multiple seed phrases. A much more ergonomic approach combining the best of
these two solutions on generalised mobile hardware could be achieved by adding
support for secp256r1 signatures.

There are already several standardised implementations of this, such as Passkeys
and WebAuthn. These solutions leverage Apple's Secure Enclave and Android Keystore
to enable users to save keypairs associated to different services natively on
the secure element of their mobile devices. To authenticate with
those services, the user uses their biometrics to sign a message with the stored
private key.

While originally intended to solve for password-less authentication in Web2
applications, WebAuthn and Passkeys also make an excellent candidate for on-chain
second-factor authentication. Beyond simply securing funds, there are also many
other potential beneficial abstractions that could make use of the simple UX
they provide.

Although WebAuthn supports the following curves:

- P-256
- P-384
- P-521
- ed25519

P-256 is the only one supported by both Android & MacOS/iOS (MacOS/iOS being the
more restrictive of the two), hence the goal being to implement secp256r1 signature
verification

General Documentation:

[WebAuthn](https://webauthn.io/)

[Passkeys](https://fidoalliance.org/passkeys/)

**Note: P-256 / secp256r1 / prime256v1 are used interchangably in this document
as they represent the same elliptic curve. The choice of nomenclature depends on
what RFC or SEC document is being referenced.**

## Alternatives Considered

We have discussed the following alternatives:

1.) Realising signature verification with a syscall similar
to `secp256k1_recover()` instead of a precompile. This would ease
integration for developers, since no instruction introspection would be
required when utilizing the syscall. This is still a valid consideration.

2.) Realising signature verification through and on-chain sBPF implemenation. On
a local validator a single signature verification consumes ≈42M compute units.
A possibility would be to split the verification into multiple transactions.
This would most probably require off-chain infrastructure to crank the process
or carry higher transaction fees for the end user. (similar to the current elusiv
protocol private transfer)
We feel this alternative directly contradicts and impinges on the main upside of
passkeys, which is the incredible UX and ease of use to the end user.

3.) Allowing for high-S signatures was considered, however the pitfalls
of signature malleability are too great to leave open to implementation.

4.) Allowing for uncompressed keys was considered, however as we are already
taking an opinionated stance on signature malleability, it makes sense to
also take an opinionated stance on public key encoding.

## New Terminology

None

## Detailed Design

The precompile's purpose is to verify signatures using ECDSA-256.
(denoted in [RFC6460](https://www.ietf.org/rfc/rfc6460.txt) as
ECDSA using the NIST P-256 curve and the SHA-256 hashing algorithm)

### Curve

The curve parameters for NIST P-256/secp256r1/prime256v1 are
outlined in the [SEC2](https://www.secg.org/SEC2-Ver-1.0.pdf#page=21)
document in Section 2.7.2

### Point Encoding/Decoding

The precompile should accept SEC1 encoded points in compressed form.
The encoding and decoding of these is outlined in sections
`2.3.3 Elliptic-Curve-Point-to-Octet-String Conversion`
and `2.3.4 Octet-String-to-Elliptic-Curve-Point Conversion`
found in [SEC1](https://www.secg.org/sec1-v2.pdf#page=16).

The SEC1 encoded EC point P = (x_p, y_p) in compressed form consists
of 33 bytes (octets). The first byte of 02_16 / 03_16 signifies a
compressed point, as well as whether y_p is odd or even. The remaining
32 bytes represent x_p converted into a 32 octet string.

While SEC1 encoded uncompressed points could also be used,
due to their larger size of 65 bytes, the ease of transformation
between uncompressed and compressed points, and the vast majority
of applications exclusively making use of compressed points, it
seems a reasonable consideration to save 32 bytes of instruction
data with a protocol that only accepts compressed points.



### ECDSA / Signature Verification

The precompile should implement the `Verifying Operation` outlined in
[SEC1](https://www.secg.org/sec1-v2.pdf#page=52)
in Section 4.1.4 as well as in the
[Digital Signature Standard (DSS)](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-5.pdf#page=36)
document in Section 6.4.2.

A multitude of test vectors to verify correctness can
be found in
[RFC6979](https://datatracker.ietf.org/doc/html/rfc6979#appendix-A.2.5)
in Section A.2.5 as well as at the
[NIST CAVP](https://csrc.nist.gov/Projects/cryptographic-algorithm-validation-program/digital-signatures#ecdsa2vs)
(Cryptographic Algorithm Validation Program)

### Signature Malleability

Due to X axis symmetry along the elliptic curve, for any ECDSA signature
`(r, s)`, there also exists a valid signature `(r, n - s)`, where `n` is the
order of the curve. This introduces "s malleability", allowing an attacker
to produce an alternative version of `s` without invalidating the signature.

The pitfalls of this in authentication systems can be particularly perilous,
opening up certain implementations to signature replay attacks over the same
message by simply flipping the `s` value over the curve.

As the primary goal of the `secp256r1` program is secure signature validation
for authentication purposes, the precompile should mitigate these attacks
by enforcing the usage of `lowS` values, in which `s <= n/2`.

As such, the program should immediately fail upon the detection of any
signature that includes a `highS` value. This prevents any accidental
succeptibility to signature malleability attacks.

Note: The existing `secp256k1` precompile makes no attempt attempt to mitigate
s malleability, as doing so would go against its primary goal of achieving
`ecrecover` parity with EVM.

### Program

ID: `Secp256r1SigVerify1111111111111111111111111`

The program instruction will be composed of the following:

- A first u8 as the count for the number of signatures to check
- Single byte of padding
- The following struct serialized, for each signature to verify

```rust
struct Secp256r1SignatureOffsets {
    signature_offset: u16,             // offset to secp256r1 signature of 64 bytes
    signature_instruction_index: u16,  // instruction index to find signature
    public_key_offset: u16,            // offset to compressed public key of 33 bytes
    public_key_instruction_index: u16, // instruction index to find public key
    message_data_offset: u16,          // offset to start of message data
    message_data_size: u16,            // size of message data
    message_instruction_index: u16,    // index of instruction data to get msg data
}
```

Multiple signatures can be verified. If any of the signatures fail to verify,
an error is returned.

The program logic will be constructed and built using a `verify`
function, as outlined in
[sdk/src/precompiles.rs](https://github.com/solana-labs/solana/blob/9ffbe2afd8ab5b972c4ad87d758866a3e1bb87fb/sdk/src/precompiles.rs).

Apart from the signature verification, the remaining
logic should be constructed analogously to the existing
[ed25519](https://github.com/solana-labs/solana/blob/master/sdk/src/ed25519_instruction.rs)
& [secp256k1](https://github.com/solana-labs/solana/blob/9ffbe2afd8ab5b972c4ad87d758866a3e1bb87fb/sdk/src/secp256k1_instruction.rs#L4)
precompiles.

### Implementation

#### Previous Consideration in SIMD-0048

The precompile could be implemented using
the `p256` crate at version `0.10.1`. This crate is part of the `Rust Crypto`
library and implements the NIST P-256 curve as well as ECDSA in native Rust.
It conforms with the test vectors found in
[RFC6979](https://datatracker.ietf.org/doc/html/rfc6979#appendix-A.2.5).

#### SIMD-0075 Update

Due to the unaudited and somewhat unknown nature of the `p256` crate and
the strict security and reproducibility considerations required to enable
compatibility with other clients, we propose to implement the precompile utilizing
the `OpenSSL` [crate](https://crates.io/crates/openssl/0.10.57).
The `OpenSSL` crate is already a dependency in the Anza client and has
additionally been heavily scrutinized/tested by the broader public.

Our benchmarks also show that verifying a signature using the `OpenSSL` crate is
~3x faster than using the `p256` crate.

Signature verification using the OpenSSL crate includes the following steps:

1. Getting the curve order using the Nid::X9_62_PRIME256V1 identifier:

   ```rust
   let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)
   ```

2. Ensuring the `r` & `s` signature components fall within `curve_order - 1`
3. Recreating the signature using the `r` & `s` signature components:

   ```rust
   let ecdsa_sig = openssl::ecdsa::EcdsaSig::from_private_components(r_bignum, s_bignum)

   let der_sig = ecdsa_sig.to_der()
   ```

4. Computing the `half_order`of the curve and ensuring that `s < half_order`
   (LowS Check)
5. Parsing the public key bytes:

   ```rust
   let ec_point = EcPoint::from_bytes(&group, pubkey, &mut ctx)

   let ec_key = EcKey::from_public_key(&group, &ec_point)

   let pkey = PKey::from_ec_key(ec_key)
   ```

6. Creating a verfier from the public key and the OpenSSL SHA-256 hashing idendifier:

   ```rust
   Verifier::new(openssl::hash::MessageDigest::sha256(), &pkey)
   ```

7. Passing the message bytes to the verifier:

   ```rust
   verifier.update(message)
   ```

8. Verify signature across the message:

   ```rust
   verifier.verify(&der_sig)
   ```

### Compute Cost / Efficiency

Once the implementation is finished, benchmarking should take place on a
sufficiently powerful machine in order to determine average compute time per
signature. Calculation of CUs would be based on the 1 CU / ns convention.
The secp256k1 ecrecover syscall, which incurs a cost of 25_000 CUs, can be used
as a reference point.

Since precompiles don't incur a flat compute cost like syscalls, this comparison
will just serve as a confirmation that the computation inside the precompile is
sufficiently efficient.

This is in line with how previous precompiles for EC group operations and
arithmetic were evaluated/benchmarked.
See [PR#27961](https://github.com/solana-labs/solana/pull/27961) & [PR#28503](https://github.com/solana-labs/solana/pull/28503)

## Impact

Would enable the on-chain usage of Passkeys and the WebAuthn Standard, and
turn the vast majority of modern smartphones into native hardware wallets.

By extension, this would also enable the creation of account abstractions and
forms of Two-Factor Authentication around those keypairs.

## Security Considerations

As multiple other clients are being developed, it is imperative that there can
be bit-level reproducibility between the precompile implementations, especially
with regard to cryptographic operations. Any discrepancy between implementations
could cause a fork and or a chain halt.

As such we would propose the following:

- Development of a thorough test suite that includes all test vectors as well
  as tests from the
  [Wycheproof Project](https://github.com/google/wycheproof#project-wycheproof)

- Creating a map of what underlying OpenSSL calls get added to the runtime when
  using the Rust bindings

## Backwards Compatibility

Transactions using the instruction could not be used on Solana versions which don't
implement this feature. A Feature gate should be used to enable this feature
when the majority of the cluster is using the required version. Transactions
that do not use this feature are not impacted.

```

```
