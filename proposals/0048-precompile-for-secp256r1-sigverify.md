---
simd: "0048"
title: Precompile for verifying secp256r1 sig.
authors:
  - Orion (Bunkr)
  - Jstnw (Bunkr)
category: Standard
type: Core
status: Draft
created: 2023-05-14
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Adding a Precompile to support the verification of signatures
generated on the secp256r1 curve.
Analogous to the support for secp256k1 and ed25519 signatures that already
exists in form of
the `KeccakSecp256k11111111111111111111111111111` and 
`Ed25519SigVerify111111111111111111111111111`
precompiles

## Motivation

Solana should have option to secure your funds in a self custodial manner that
doesn't just airgap your private key with a hardware wallet (which even then
remains as a single point of failure). Arguably, multi-signature wallets fit
into this equation as they enable the dependency on multiple private keys.
However in practice the UX takes too much of a hit, as having to sign a
transaction a minimum of 3 separate times and having to write down 3 seed
phrases is too cumbersome. It would be ideal to have an authentication form that
relies on a more familiar second factor, such as a users mobile device.

Passkeys & WebAuthn are a standardized implementation of this. They enable users
to save keypairs associated to different services natively on the secure
element of their mobile device. To authenticate with those services, the user
uses their biometrics to sign a message with the stored private key.

And although this is meant to enable password-less logins in web2, it makes for
an excellent candidate as a second factor of on-chain authentication.

Going past just securing funds, this would support other beneficial account
abstractions that make use of the simple UX of WebAuthn and Passkeys.

Note:

Although WebAuthn supports the following curves:

- P-256
- P-384
- P-521
- ed25519

P-256 is the only one supported by both Android & IOS (IOS being the more
restrictive of the two), hence the goal being to implement secp256r1 signature
verification

General Documentation:

[WebAuthn](https://webauthn.io/)

[Passkeys](https://fidoalliance.org/passkeys/)

## Alternatives Considered

We have discussed the following alternatives:

1.) Realising signature verification with a syscall similar
to `secp256k1_recover()` instead of a precompile. This would ease
integration for developers, since no instruction introspection would be
required when utilizing the syscall.

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

### Point Encoding/Decoding:

The precompile should accept SEC1 encoded points in compressed form.
The encoding and decoding of these is outlined in sections 
`2.3.3 Elliptic-Curve-Point-to-Octet-String Conversion` 
and `2.3.4 Octet-String-to-Elliptic-Curve-Point Conversion`
found in [SEC1](https://www.secg.org/sec1-v2.pdf#page=16).

The SEC1 encoded EC point P = (x_p, y_p) 
in compressed form consists of 33 bytes (octets). 
The first byte 02_16 / 03_16 signifies
whether the point is compressed or uncompressed as well as 
signifying the odd or even state of y_p. The 
remaining 32 bytesrepresent x_p converted 
into a 32 octet string.

SEC1 endcoded uncompressed points, which consist of 65 bytes, 
have been deliberately disregarded as y_p is not needed
during signature verification and it seems sensible to save 32 
bytes of transaction space.

**Note:** The existing precompiles for secp256k1 & ed25519 utilize 
just x_p encoded as an octet string. This saves one byte 
compared to using a compressed point, but fails to conform to any standard.

### ECDSA / Signature Verification

The precompile should implement the `Verifying Operation` outlined in 
[SEC1](https://www.secg.org/sec1-v2.pdf#page=52)
in Section 4.1.4 as well as in the 
[Digital Signature Standard (DSS)](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-5.pdf#page=36)
document in Section 6.4.2.

A multitude of test vectors to verify correctness can 
be found in [RFC6979](https://datatracker.ietf.org/doc/html/rfc6979#appendix-A.2.5) 
in Section A.2.5 as well as at the 
[NIST CAVP](https://csrc.nist.gov/Projects/cryptographic-algorithm-validation-program/digital-signatures#ecdsa2vs)
(Cryptographic Algorithm Validation Program)


### Signature Malleability

As any signature `s = (R,S)` generated with ECDSA is malleable 
in regards to the `S` value, the precompile should enforce the usage
of `lowS` values, in which `S < n/2` where `n` is the order of 
the elliptic curve.
It should fail on any signatures that include a `highS` value.

This should done to prevent any accidental succeptibility to
signature malleability attacks.

Note: The existing secp256k1 precompile does not prevent signature malleability

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

The precompile can be implemented using the `p256` crate at version `0.10.1`.
This crate is part of the `Rust Crypto` library and implements
the NIST P-256 curve as well as ECDSA.
It conforms with the test vectors found in [RFC6979](https://datatracker.ietf.org/doc/html/rfc6979#appendix-A.2.5).

The precompile would make use of the following to accomplish signature
verification:

- `p256::ecdsa::VerifyingKey::from_sec1_bytes()`
- `p256::ecdsa::Signature::from_scalars()`
- `p256::arithmetic::Scalar::is_high()`
- `p256::ecdsa::VerifyingKey::verify()`

Note: The crate is well maintained, but has never been externally audited.

### Compute Cost

Once the implementation is finished, benchmarking should take place on a
sufficiently powerful machine in order to determine average compute time.
Pricing would be based on the 33ns/CU convention. For
the sake of ensuring proper efficiency, a comparison to similar implementations
on polygon/optimism/ethereum would be conducted.

This is in line with how previous precompiles for EC group operations and
arithmetic were evaluated/benchmarked.
See [PR#27961](https://github.com/solana-labs/solana/pull/27961) & [PR#28503](https://github.com/solana-labs/solana/pull/28503)

## Impact

Would enable the on-chain usage of Passkeys and the WebAuthn Standard.

By extension this would also enable the creation of account abstractions and
forms of Two-Factor Authentication around those keypairs.

## Security Considerations

As [Firedancer](https://github.com/firedancer-io/firedancer) is being developed
in C, it is imperative that there can be bit-level reproducibility between 
the precompile implementations. Any discrepancy between the two implementations
could cause a fork and or a chain halt. (Thank you @fd-ripatel for pointing this
out and advocating for it)

As such we would propose the following:

- Development of a thorough test suite that includes all test vectors as well as tests
from the [Wycheproof Project](https://github.com/google/wycheproof#project-wycheproof)

- Thorough auditing as well as a formal verification of the arithmetic and
decoding inside the `p256` crate

## Backwards Compatibility

Transactions using the instruction could not be used on Solana versions which don't
implement this feature. A Feature gate should be used to enable this feature
when the majority of the cluster is using the required version. Transactions
that do not use this feature are not impacted.
