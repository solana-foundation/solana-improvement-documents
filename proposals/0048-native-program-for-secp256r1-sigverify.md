---
simd: "0048"
title: Native Program for verifying secp256r1 sig.
authors:
  - Orion (Bunkr)
  - Jstnw (Bunkr)
category: Standard
type: Core
status: Withdrawn
created: 2023-05-14
---

## Summary

Adding a Native Program to support the verification of signatures
generated on the secp256r1 curve.
Analogous to the support for secp256k1 and ed25519 signatures that already
exists in form of
the `KeccakSecp256k11111111111111111111111111111` and
`Ed25519SigVerify111111111111111111111111111`
native programs

## Motivation

Solana should have option to secure your funds in a self custodial manner that
doesn't just airgap your private key with a hardware wallet (which even then
remains as a single point of failure). Arguably, multi-signature wallets fit
into this equation as they enable the dependency on multiple private keys.
However in practice the UX takes too much of a hit, as having to sign a
transaction a minimum of 3 seperate times and having to write down 3 seed
phrases is too cumbersome. It would be ideal to have an authetication form that
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

P-256 is the only one suported by both Android & IOS (IOS being the more
restrictive of the two), hence the goal being to implement secp256r1 signature
verification

General Documentation:

[WebAuthn](https://webauthn.io/)

[Passkeys](https://fidoalliance.org/passkeys/)

## Alternatives Considered

We have discussed the following alternatives:

1.) Realising signature verification with a syscall similar
to `secp256k1_recover()` instead of a native program. This would ease
integration for developers, since no instruction introspection would be
required when utilizing the syscall.

## New Terminology

None

## Detailed Design

Implementation would be as follows:

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
    public_key_offset: u16,            // offset to public key of 32 bytes
    public_key_instruction_index: u16, // instruction index to find public key
    message_data_offset: u16,          // offset to start of message data
    message_data_size: u16,            // size of message data
    message_instruction_index: u16,    // index of instruction data to get msg data
}
```

Multiple signatures can be verified. If any of the signatures fail to verify,
an error is returned.

Should be analogous to `KeccakSecp256k11111111111111111111111111111`
and `Ed25519SigVerify111111111111111111111111111`.
View reference details at [sdk/src/ed25519_instruction.rs](https://github.com/solana-labs/solana/blob/master/sdk/src/ed25519_instruction.rs)

### Implementation

The crates `ecdsa` and `p256` are a good starting point for the implementation.
Due to a current dependency version conflict of `zeroize` between
`curve25519-dalek` and `solana-program`, using these crates will require a
fix/bump of `zeroize` inside `curve25519-dalek`. See issue [#26688](https://github.com/solana-labs/solana/issues/26688)

### Compute Cost

Once the implementation is finished, benchmarking should take place on a
sufficiently powerful machine in order to determine average compute time.
Pricing would be based on the 33ns/CU convention. For
the sake of ensuring proper efficiency, a comparison to similar implementations
on polygon/optimism/ethereum would be conducted.

This is in line with how previous native programs for EC group operations and
arithmetic were evaluated/benchmarked.
See [PR#27961](https://github.com/solana-labs/solana/pull/27961) & [PR#28503](https://github.com/solana-labs/solana/pull/28503)

## Impact

Would enable the on-chain usage of Passkeys and the WebAuthn Standard.

By extension this would also enable the creation of account abstractions and
forms of Two-Factor Authentication around those keypairs.

## Security Considerations

- Ensure parity of test results and parameters with those found in
  [SEC2](https://www.secg.org/sec2-v2.pdf) for the secp256r1 curve
- Ensure signature malleability is prevented/accounted for

## Backwards Compatibility

Transactions using the instruction could not be used on Solana versions which don't
implement this feature. A Feature gate should be used to enable this feature
when the majority of the cluster is using the required version. Transactions
that do not use this feature are not impacted.
