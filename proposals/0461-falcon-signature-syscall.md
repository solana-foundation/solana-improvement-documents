---
simd: '0461'
title: Falcon-512 Signature Syscall
authors:
  - ZZ
category: Standard
type: Core
status: Idea
created: 2026-01-16
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Adding a syscall to support the verification of Falcon-512 signatures,
providing post-quantum cryptographic security for Solana transactions.
This enables quantum-resistant signature verification as an alternative
to the existing Ed25519 signatures.

## Motivation

Cryptographically Relevant Quantum Computers (CRQCs) pose a significant
threat to the security of current elliptic curve and RSA-based
cryptographic systems. When sufficiently powerful quantum computers become
available, Shor's algorithm will be able to break Ed25519, secp256k1, and
other elliptic curve signatures currently used by Solana and other
blockchains.

NIST has standardized several post-quantum cryptographic algorithms
as part of their Post-Quantum Cryptography Standardization process.
Falcon (FN-DSA) was selected for standardization as draft FIPS 206,
with final publication expected in late 2026 or early 2027. Falcon was
chosen for its compact signature size relative to other lattice-based
schemes, making it particularly suitable for blockchain applications
where transaction size directly impacts costs and throughput.

By adding Falcon-512 signature verification as a syscall, Solana can:

1. **Future-proof the network**: Enable users to protect high-value
   accounts and critical infrastructure against future quantum attacks.

2. **Support hybrid security models**: Allow applications to require both
   classical (Ed25519) and post-quantum (Falcon) signatures for
   defense-in-depth.

3. **Enable gradual migration**: Provide a migration path for the
   ecosystem to transition to quantum-resistant cryptography before
   quantum computers become a practical threat.

4. **Maintain competitive positioning**: Other blockchain networks
   (Ethereum via EIP-8052, etc.) are also preparing post-quantum
   solutions. Solana should not fall behind in cryptographic security.

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in RFC 2119.

## Dependencies

None.

## New Terminology

- **Falcon / FN-DSA**: A lattice-based digital signature algorithm
  selected by NIST for standardization (draft FIPS 206). Based on the
  "Fast Fourier Lattice-based Compact Signatures over NTRU" scheme.

- **Falcon-512**: The smaller parameter set of Falcon, providing
  approximately 128 bits of classical security and targeting NIST
  Security Level I (equivalent to AES-128).

- **NTRU lattice**: The algebraic structure underlying Falcon, based on
  polynomial rings modulo q = 12289.

- **Post-quantum cryptography (PQC)**: Cryptographic algorithms believed
  to be secure against attacks by both classical and quantum computers.

## Detailed Design

The syscall's purpose is to verify Falcon-512 signatures in accordance
with the Falcon specification (v1.2).

### Falcon-512 Parameters

The following parameters define Falcon-512:

| Parameter | Value |
|-----------|-------|
| n (degree) | 512 |
| q (modulus) | 12289 |
| Public key size | 897 bytes |
| Signature size | 666 bytes (padded, fixed) |
| Security level | NIST Level I (~128 bits classical) |

### Syscall

We propose a new syscall that verifies a single Falcon-512 signature per call:

```rust
define_syscall!(fn sol_falcon512_verify(
    signature_addr: *const u8, // 666 bytes, fixed-length padded signature
    public_key_addr: *const u8, // 897 bytes
    message_addr: *const u8,
    message_len: u64
) -> u64);
```

Programs can batch verifications by invoking the syscall multiple times.

Note: This proposal uses the fixed-length (padded) Falcon-512 signature
encoding. Signatures MUST be exactly 666 bytes, as specified in the
Falcon v1.2 specification.

### Signature Format

Falcon-512 signatures MUST be in the fixed-length (padded) format as
specified in the Falcon v1.2 specification. The signature consists of:

1. A header byte (Falcon-512 padded: `0x39`)
2. A 40-byte random salt (nonce)
3. The padded encoding of the signature polynomial s2

The syscall MUST treat signatures as invalid if they:

- Are provided with insufficient buffer length (the syscall always reads
  exactly 666 bytes)
- Have an invalid header byte
- Fail decoding
- Have coefficients outside the valid range

### Public Key Format

Public keys MUST be in the format specified in the Falcon v1.2
specification:

1. A header byte (0x09 for Falcon-512, computed as 0x00 + logn where logn = 9)
2. The 896-byte encoding of the public key polynomial h

Total public key size: 897 bytes

The syscall MUST treat public keys as invalid if they:

- Are provided with insufficient buffer length (the syscall always reads
  exactly 897 bytes)
- Have an invalid header byte
- Fail decoding

### Verification Algorithm

The verification follows the Falcon v1.2 verification algorithm:

1. Decode the public key h from the encoded format
2. Decode the signature (salt, s2) from the padded format
3. Compute c = HashToPoint(salt || message)
4. Compute s1 = c - s2 * h (mod q)
5. Verify that $$||(s1, s2)||^2 <= bound^2$$

The HashToPoint function uses SHAKE-256 as specified in the Falcon v1.2
to hash the message into a polynomial in $$\mathbb{Z}_q[x]/(x^n + 1)$$.
The `bound` value is the Falcon-512 norm bound parameter defined in
the Falcon v1.2 specification; implementations MUST use that exact
value for verification.
Domain separation for HashToPoint MUST follow the Falcon v1.2
definition for Falcon-512. If Solana-specific domain separation is
desired, it MUST be applied by the caller to the message bytes before
invoking the syscall.

### Behavior

The syscall MUST:

1. Read exactly 666 bytes at `signature_addr` and 897 bytes at
   `public_key_addr`.
2. Read `message_len` bytes at `message_addr`.

The syscall MUST treat any signature or public key that fails decoding
or format validation as invalid.

Return values:

- `0`: signature is valid
- `1`: signature is invalid (including any decoding or format failure)

The syscall MUST abort the virtual machine if any of the following are
true:

- The VM memory ranges `[signature_addr, signature_addr + 666)` or
  `[public_key_addr, public_key_addr + 897)` are not readable.
- The VM memory range `[message_addr, message_addr + message_len)` is
  not readable.
- `message_len` exceeds `MAX_FALCON_MESSAGE_LEN`.
- Any pointer + length arithmetic overflows.
- Compute budget is exceeded.

### Compute Cost

(**Tentative**)

Falcon-512 verification is computationally more expensive than Ed25519
due to the polynomial arithmetic involved. Benchmarking MUST be performed
on representative hardware to determine appropriate compute costs.

Based on preliminary estimates from reference implementations:

- Falcon-512 verification: approximately 10-20µs on modern hardware
- This translates to approximately 15,000-30,000 CUs per verification

Final compute costs MUST be determined through benchmarking in accordance
with established Solana compute unit pricing conventions (33ns/CU).

Because the verification hashes `message_len` bytes via SHAKE-256, the
syscall MUST meter compute cost for hashing work proportionally to
`message_len`, or enforce a maximum `message_len` to cap total cost.
This proposal specifies both:

- `MAX_FALCON_MESSAGE_LEN`: maximum allowed `message_len` (bytes). Calls
  with larger values MUST abort the VM.
- `falcon_hash_bytes_per_cu`: bytes per CU charged for hashing work.

Compute cost MUST be charged as:

```
falcon_verify_base
+ ceil(message_len / falcon_hash_bytes_per_cu)
```

Where `falcon_verify_base` and `falcon_hash_bytes_per_cu` are runtime
parameters set by benchmarking. `MAX_FALCON_MESSAGE_LEN` is a fixed
constant defined by this SIMD and enforced by the feature gate.
**Tentative**: `MAX_FALCON_MESSAGE_LEN = 65_535`.
Cost calculation MUST use saturating arithmetic and MUST NOT fail due to
overflow.

Programs can invoke the syscall multiple times to verify multiple
signatures within a single instruction; the total compute budget will
apply across all invocations.

### Syscall Integration

- The syscall MUST be feature-gated and unavailable prior to activation.
- The syscall MUST be registered in the runtime syscall table under the
  name `sol_falcon512_verify`.

## Alternatives Considered

### 1. Other Post-Quantum Signature Schemes

**Dilithium (ML-DSA):**

- Pros: Larger security margins, simpler implementation
- Cons: Significantly larger signatures (~2,420 bytes for Dilithium2 vs
  ~666 bytes for Falcon-512), making it less suitable for blockchain use

**SPHINCS+:**

- Pros: Hash-based, very conservative security assumptions
- Cons: Very large signatures (~17KB-49KB), impractical for blockchain

**Falcon-1024:**

- Pros: Higher security level (NIST Level V)
- Cons: Larger signatures (~1,280 bytes) and keys (~1,793 bytes),
  higher computational cost

Falcon-512 provides the best balance of security, signature size, and
verification performance for blockchain applications.

### 2. Precompile Instead of Syscall

Following the existing signature verification pattern (Ed25519 and
secp256k1) via precompiles would keep consistency with established
native programs. However, a syscall avoids instruction introspection,
is simpler for program developers, and aligns with other cryptographic
operations already exposed via syscalls.

### 3. On-chain BPF Implementation

Implementing Falcon verification in BPF would consume excessive compute
units due to the complex polynomial arithmetic involved, making it
impractical without syscall support.

### 4. Hash Function Variants

EIP-8052 proposes two variants: one using SHAKE-256 (NIST-compliant) and
one using Keccak256 (EVM-optimized). For Solana, we recommend only the
NIST-compliant SHAKE-256 variant to:

- Maintain compliance with draft FIPS 206
- Avoid unnecessary complexity
- Ensure interoperability with other draft FIPS 206 implementations

## Impact

### For dApp Developers

- New syscall available for post-quantum signature verification
- Larger signature and public key sizes require adjustments to account
  data structures and transaction layouts
- SDK updates will be needed to support Falcon key generation and signing

### For Validators

- Additional syscall to implement and maintain
- Higher computational cost per Falcon signature verification
- No impact on existing transaction processing

### For Core Contributors

- Implementation of Falcon-512 verification following draft FIPS 206
- Integration with the syscall interface and compute budget accounting
- Development of comprehensive test suites for cross-client compatibility

## Security Considerations

### Algorithm Security

Falcon-512 targets NIST Security Level I, providing approximately 128
bits of classical security and resistance against known quantum attacks.
The security is based on the hardness of the **NTRU problem**, which
relates to finding short vectors (**SVP**) in NTRU lattices. Without
knowledge of the trapdoor (private key), an attacker cannot find the
short signature vectors that satisfy the verification equation.

### Implementation Security

1. **Constant-time implementation**: Constant-time implementation is NOT
   required for verification. Unlike signing (which involves secret key
   operations), verification only processes public data (public key,
   message, signature) and does not leak secret information through
   timing variations.

2. **Input validation**: All inputs (signatures, public keys) MUST be
   thoroughly validated before processing to prevent malformed input
   attacks.

3. **Signature uniqueness and malleability**: The random salt (nonce)
   makes honest signatures unique across signings of the same message.
   Non-malleability relies on the verification rule, including the
   tight norm bound check, which should reject transformed signatures.

### Cross-Client Consistency

As with other syscalls, it is imperative that there is bit-level
reproducibility between implementations across different validator
clients. Any discrepancy could cause network forks.

Recommendations:

- Development of a comprehensive test suite including NIST test vectors
- Active communication between client teams during implementation
- Use of well-audited reference implementations where possible

### Migration Considerations

This syscall does NOT replace Ed25519 for transaction signing. It
provides an additional verification capability that applications can
choose to use. A full migration of Solana's core transaction signatures
to post-quantum algorithms would require a separate, more comprehensive
proposal.

## Drawbacks

1. **Larger data sizes**: Falcon-512 public keys (897 bytes) and
   signatures (~666 bytes) are significantly larger than Ed25519 (32
   bytes and 64 bytes respectively), increasing transaction sizes and
   storage costs.

2. **Higher computational cost**: Falcon verification is more expensive
   than Ed25519, potentially reducing transaction throughput if widely
   adopted.

3. **Complexity**: Falcon's lattice-based cryptography is more complex
   than elliptic curve cryptography, potentially increasing the attack
   surface and maintenance burden.

4. **Uncertain timeline**: The threat from quantum computers remains
   uncertain, and this capability may not be needed for many years.

5. **Ecosystem immaturity**: Post-quantum cryptography tooling and
   libraries are less mature than classical cryptography.

6. **Specification alignment**: If future NIST standardization (FIPS 206)
   diverges from the Falcon v1.2 specification referenced here, this
   syscall may require updates before activation. The implementation
   SHOULD track the NIST standardization process and incorporate any
   changes needed for alignment.

## Backwards Compatibility

Transactions using the Falcon syscall cannot be processed
on Solana versions that do not implement this feature. A feature gate
MUST be used to enable this feature when the majority of the cluster is
running the required version.

Transactions that do not use this feature are not impacted.

## Scope

This syscall provides **signature verification only**. Key generation
and signing are out of scope and must be performed off-chain. Note that
Falcon signing is more complex than Ed25519 signing, requiring discrete
Gaussian sampling over lattices. Implementers should use well-audited
cryptographic libraries for signing operations.

## Test Vectors

Implementations MUST pass the Known Answer Tests (KATs) provided by NIST
for FN-DSA. Additional test vectors will be derived from:

- NIST ACVP (Automated Cryptographic Validation Protocol) test vectors
- Falcon reference implementation test suite
- Wycheproof project test vectors (when available)

A comprehensive test suite will be developed and maintained to ensure
cross-client compatibility.

## References

- [Falcon Specification v1.2 (PDF)][falcon-spec]
- [FIPS 206: FN-DSA (Falcon) Presentation][fips-206] - NIST CSRC presentation
- [FIPS 206 Status Update (PDF)][fips-206-pdf]
- [EIP-8052: Falcon Signature Precompile][eip-8052]
- [SIMD-0075: Precompile for secp256r1][simd-0075]
- [Falcon Reference Implementation](https://falcon-sign.info/)
- [NIST PQC Standardization](https://csrc.nist.gov/projects/post-quantum-cryptography)

[falcon-spec]: https://falcon-sign.info/falcon.pdf
[fips-206]: https://csrc.nist.gov/presentations/2025/fips-206-fn-dsa-falcon
[fips-206-pdf]: https://csrc.nist.gov/csrc/media/presentations/2025/fips-206-fn-dsa-%28falcon%29/images-media/fips_206-perlner_2.1.pdf
[eip-8052]: https://eips.ethereum.org/EIPS/eip-8052
[simd-0075]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0075-precompile-for-secp256r1-sigverify.md
