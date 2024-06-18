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

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL
NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and
"OPTIONAL" in this document are to be interpreted as described in
RFC 2119.

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

Apart from the RFC mandated implementation the precompile must additionally take
an opinionated stance on signature malleability.

### Signature Malleability

Due to X axis symmetry along the elliptic curve, for any ECDSA signature
`(r, s)`, there also exists a valid signature `(r, n - s)`, where `n` is the
order of the curve. This introduces "s malleability", allowing an attacker
to produce an alternative version of `s` without invalidating the signature.

The pitfalls of this in authentication systems can be particularly perilous,
opening up certain implementations to signature replay attacks over the same
message by simply flipping the `s` value over the curve.

As the primary goal of the `secp256r1` program is secure signature validation
for authentication purposes, the precompile must mitigate these attacks
by enforcing the usage of `lowS` values, in which `s <= n/2`.

As such, the program must immediately fail upon the detection of any
signature that includes a `highS` value. This prevents any accidental
succeptibility to signature malleability attacks.

Note: The existing `secp256k1` precompile makes no attempt attempt to mitigate
s malleability, as doing so would go against its primary goal of achieving
`ecrecover` parity with EVM.

### Implementation

### Program

ID: `Secp256r1SigVerify1111111111111111111111111`

In accordance with [SIMD
0152](https://github.com/solana-foundation/solana-improvement-documents/pull/152)
the programs ```verify``` instruction must accept the following data:

In Pseudocode:

```
struct Secp256r1SigVerifyInstruction {
    num_signatures: uint8 LE,                  // Number of signatures to verify
    padding: uint8 LE,                         // Single byte padding
    offsets: Array<Secp256r1SignatureOffsets>, // Array of offset structs
    additionalData?: Bytes,                    // Optional additional data, e.g.
                                               // signatures included in the same
                                               // instruction
}
Note: Array<Secp256r1SignatureOffsets> does not contain any length prefixes or
padding between elements.

struct Secp256r1SignatureOffsets {
    signature_offset: uint16 LE,              // Offset to signature
    signature_instruction_index: uint16 LE,   // Instruction index to signature
    public_key_offset: uint16 LE,             // Offset to public key
    public_key_instruction_index: uint16 LE,  // Instruction index to  public key
    message_offset: uint16 LE,                // Offset to start of message data
    message_length: uint16 LE,                // Size of message data
    message_instruction_index: uint16 LE,     // Instruction index to message
}
```

Up to 8 signatures can be verified. If any of the signatures fail to verify,
an error must be returned.

In accordance with [SIMD
0152](https://github.com/solana-foundation/solana-improvement-documents/pull/152)
the behavior of the program must be as follows:

1. If instruction `data` is empty, return error.
2. The first byte of `data` is the number of signatures `num_signatures`.
3. If `num_signatures` is 0, return error.
4. Expect (enough bytes of `data` for) `num_signatures` instances of
   `Secp256r1SignatureOffsets`.
5. For each signature:
   a. Read `offsets`: an instance of `Secp256r1SignatureOffsets`
   b. Based on the `offsets`, retrieve `signature`, `public_key`, and
      `message` bytes. If any of the three fails, return error.
   c. Invoke the actual `sigverify` function. If it fails, return error.

To retrieve `signature`, `public_key`, and `message`:

1. Get the `instruction_index`-th `instruction_data`
   - The special value `0xFFFF` means "current instruction"
   - If the index is invalid, return Error
2. Return `length` bytes starting from `offset`
   - If this exceeds the `instruction_data` length, return Error

Note that fields (offsets) can overlap, for example the same public key or
message can be referred to by multiple instances of `Secp256r1SignatureOffsets`.

If the precompile `verify` function returns any error, the whole transaction
should fail. Therefore, the type of error is irrelevant and is left as an
implementation detail.

The instruction processing logic must follow the pseudocode below:

```
/// `data` is the secp256r1 program's instruction data. `instruction_datas` is
/// the full slice of instruction datas for all instructions in the transaction,
/// including the secp256r1 program's instruction data.

/// length_of_data is the length of `data`

/// SERIALIZED_OFFSET_STRUCT_SIZE is the length of the serialized
/// Secp256r1SignatureOffsets struct

/// SERIALIZED_PUBLIC_KEY_LENGTH and SERIALIZED_SIGNATURE_LENGTH represent the 
/// length of the serialized public key and signature respectively

function verify() {
  if length_of_data == 0 {
    return Error
  }
  num_signatures = data[0]
  if num_signatures == 0 && length_of_data > 1 {
    return Error
  }
  if length_of_data < (num_signatures * SERIALIZED_OFFSET_STRUCT_SIZE + 2) {
    return Error
  }
  all_tx_data = { data, instruction_datas }
  data_start_position = 2

  for i in 0..num_signatures {
      offsets = (Secp256r1SignatureOffsets) 
        all_tx_data.data[data_start_position..data_start_position + SERIALIZED_OFFSET_STRUCT_SIZE]
      data_position += SERIALIZED_OFFSET_STRUCT_SIZE

      signature = get_data_slice(all_tx_data,
                                offsets.signature_instruction_index,
                                offsets.signature_offset
                                signature_length)
      if !signature {
        return Error
      }

      public_key = get_data_slice(all_tx_data,
                                  offsets.public_key_instruction_index,
                                  offsets.public_key_offset,
                                  SERIALIZED_PUBLIC_KEY_LENGTH)
      if !public_key {
        return Error
      }

      message = get_data_slice(all_tx_data,
                              offsets.message_instruction_index,
                              offsets.message_offset
                              offsets.message_length)
      if !message {
        return Error
      }

      // sigverify includes validating signature and public_key
      // the additional highS check is done here
      if signature_S == highS {
      return Error
      }
      result = sigverify(signature, public_key, message)
      if result != Success {
        return Error
      }
    }
    return Success
}
// This function is re-used across precompiles in accordance with SIMD-0152
fn get_data_slice(all_tx_data, instruction_index, offset, length) {
  // Get the right instruction_data
  if instruction_index == 0xFFFF {
    instruction_data = all_tx_data.data
  } else {
    if instruction_index >= num_instructions {
      return Error
    }
    instruction_data = all_tx_data.instruction_datas[instruction_index]
  }

  start = offset
  end = offset + length
  if end > instruction_data_length {
    return Error
  }

  return instruction_data[start..end]
}    
```

Additonally the precompile's core `verify` function must be constructed in
accordance with the structure outlined in [sdk/src/precompiles.rs](https://github.com/solana-labs/solana/blob/9ffbe2afd8ab5b972c4ad87d758866a3e1bb87fb/sdk/src/precompiles.rs).

### Compute Cost / Efficiency

Benchmarking and compute cost calculations must be done in accordance with [SIMD-0121](https://github.com/solana-foundation/solana-improvement-documents/pull/121)

Additionally, comparisons to existing precompiles should be done to check for
comperable efficiency.

## Impact

Would enable the on-chain usage of Passkeys and the WebAuthn Standard, and
turn the vast majority of modern smartphones into native hardware wallets.

By extension, this would also enable the creation of account abstractions and
forms of Two-Factor Authentication around those keypairs.

## Security Considerations

The following security considerations must be made for the
implementation of ECDSA over NIST P-256.

### Curve

The curve parameters for NIST P-256/secp256r1/prime256v1 are
outlined in the [SEC2](https://www.secg.org/SEC2-Ver-1.0.pdf#page=21)
document in Section 2.7.2

### Point Encoding/Decoding

The precompile must accept SEC1 encoded points in compressed form.
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

The precompile must implement the `Verifying Operation` outlined in
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

### General

As multiple other clients are being developed, it is imperative that there is
bit-level reproducibility between the precompile implementations, especially
with regard to cryptographic operations. Any discrepancy between implementations
could cause a fork and or a chain halt.

As such we would propose the following:

- Development of a thorough test suite that includes all test vectors as well
  as tests from the
  [Wycheproof Project](https://github.com/google/wycheproof#project-wycheproof)

- Maintaining active communication with other clients to ensure parity and to
  support potential changes if they arise.

## Backwards Compatibility

Transactions using the instruction could not be used on Solana versions which don't
implement this feature. A Feature gate should be used to enable this feature
when the majority of the cluster is using the required version. Transactions
that do not use this feature are not impacted.
