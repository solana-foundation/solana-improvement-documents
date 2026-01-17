---
simd: 'XXXX'
title: Precompile for Falcon-512 Signature Verification
authors:
  - TBD
category: Standard
type: Core
status: Draft
created: 2026-01-16
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Adding a precompile to support the verification of Falcon-512 signatures,
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

By adding Falcon-512 signature verification as a precompile, Solana can:

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

This proposal depends on the following previously accepted proposals:

- **[SIMD-0152]: Precompiles**

    This SIMD follows the unified precompile behavior defined in SIMD-0152.

[SIMD-0152]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0152-precompiles.md

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

The precompile's purpose is to verify Falcon-512 signatures in accordance
with draft FIPS 206 (FN-DSA).

### Falcon-512 Parameters

The following parameters define Falcon-512:

| Parameter | Value |
|-----------|-------|
| n (degree) | 512 |
| q (modulus) | 12289 |
| Public key size | 897 bytes |
| Signature size | <= 666 bytes |
| Security level | NIST Level I (~128 bits classical) |

### Program

ID: `Fa1con512SigVerify11111111111111111111111111`

In accordance with [SIMD-0152], the program's `verify` instruction MUST
accept the following data:

```
struct Falcon512SigVerifyInstruction {
    num_signatures: uint8 LE,                  // Number of signatures to verify
    padding: uint8 LE,                         // Single byte padding
    offsets: Array<Falcon512SignatureOffsets>, // Array of offset structs
    additionalData?: Bytes,                    // Optional additional data
}

struct Falcon512SignatureOffsets {
    signature_offset: uint16 LE,               // Offset to signature
    signature_length: uint16 LE,               // Length of signature (variable)
    signature_instruction_index: uint16 LE,    // Instruction index to signature
    public_key_offset: uint16 LE,              // Offset to public key
    public_key_instruction_index: uint16 LE,   // Instruction index to public key
    message_offset: uint16 LE,                 // Offset to start of message data
    message_length: uint16 LE,                 // Size of message data
    message_instruction_index: uint16 LE,      // Instruction index to message
}
```

Note: Unlike Ed25519 and secp256k1/r1 which have fixed signature sizes,
Falcon signatures have variable length due to compression. The
`signature_length` field is required to handle this variability. The
maximum compressed signature size is 666 bytes as specified in draft
FIPS 206. This value may change in the final specification; implementations
MUST be updated to match the final FIPS 206 before activation.

The padding byte MUST be ignored and MAY contain any value.

### Signature Format

Falcon-512 signatures MUST be in the compressed format as specified in
draft FIPS 206 Section 3.5. The signature consists of:

1. A header byte (0x39 for Falcon-512 compressed, computed as 0x30 + logn)
2. A 40-byte random salt (nonce)
3. The compressed signature polynomial s2

The precompile MUST reject signatures that:
- Exceed the maximum allowed signature size (666 bytes)
- Have an invalid header byte
- Fail decompression
- Have coefficients outside the valid range

### Public Key Format

Public keys MUST be in the format specified in draft FIPS 206 Section 3.3:

1. A header byte (0x09 for Falcon-512, computed as 0x00 + logn where logn = 9)
2. The 896-byte encoding of the public key polynomial h

Total public key size: 897 bytes

The precompile MUST reject public keys that:
- Have an invalid header byte
- Have an incorrect length
- Fail decoding

### Verification Algorithm

The verification follows draft FIPS 206 Algorithm 18 (Verify):

1. Decode the public key h from the encoded format
2. Decode the signature (salt, s2) from the compressed format
3. Compute c = HashToPoint(salt || message)
4. Compute s1 = c - s2 * h (mod q)
5. Verify that ||(s1, s2)||^2 <= bound^2

The HashToPoint function uses SHAKE-256 as specified in draft FIPS 206
to hash the message into a polynomial in Z_q[x]/(x^n + 1).

### Behavior

In accordance with [SIMD-0152], the behavior of the precompile MUST be:

1. If instruction `data` is empty, return error.
2. The first byte of `data` is the number of signatures `num_signatures`.
3. If `num_signatures` is 0, return error.
4. If `num_signatures` > 8, return error (MAX_ALLOWED_PRECOMPILE_SIGNATURES).
5. Expect enough bytes of `data` for `num_signatures` instances of
   `Falcon512SignatureOffsets`.
6. The second byte (padding) MUST be ignored and MAY contain any value.
7. Iterate `num_signatures` times:
   a. Read `offsets`: an instance of `Falcon512SignatureOffsets`
   b. Based on the `offsets`, retrieve `signature`, `public_key`, and
      `message` bytes. If any of the three fails, return error.
   c. Validate signature length is within bounds [41, 666]. The minimum
      of 41 bytes accounts for the header byte (1) plus salt (40). Note
      that decode failures during verification supersede length checks.
   d. Invoke the Falcon-512 verification function. If it fails, return error.

All arithmetic operations (offset + length, num_signatures * struct_size,
data_start_position increments) MUST use overflow-checked arithmetic.
If overflow occurs, return error.

```
/// Pseudocode for verification

SERIALIZED_OFFSET_STRUCT_SIZE = 16  // 8 uint16 fields
PUBLIC_KEY_LENGTH = 897
MAX_SIGNATURE_LENGTH = 666
MIN_SIGNATURE_LENGTH = 41  // header (1) + salt (40)

function verify() {
    if length_of_data == 0 {
        return Error
    }

    num_signatures = data[0]

    if num_signatures == 0 {
        return Error
    }

    if num_signatures > 8 {
        return Error
    }

    // Check for overflow before arithmetic
    required_length = checked_add(
        checked_mul(num_signatures, SERIALIZED_OFFSET_STRUCT_SIZE),
        2
    )
    if required_length == OVERFLOW || length_of_data < required_length {
        return Error
    }

    all_tx_data = { data, instruction_datas }
    data_start_position = 2

    // Iterate num_signatures times
    for i in 0 to num_signatures (exclusive) {
        offsets = (Falcon512SignatureOffsets)
            all_tx_data.data[data_start_position..
                             data_start_position + SERIALIZED_OFFSET_STRUCT_SIZE]
        data_start_position += SERIALIZED_OFFSET_STRUCT_SIZE

        // Validate signature length
        if offsets.signature_length < MIN_SIGNATURE_LENGTH ||
           offsets.signature_length > MAX_SIGNATURE_LENGTH {
            return Error
        }

        signature = get_data_slice(all_tx_data,
                                   offsets.signature_instruction_index,
                                   offsets.signature_offset,
                                   offsets.signature_length)
        if !signature {
            return Error
        }

        public_key = get_data_slice(all_tx_data,
                                    offsets.public_key_instruction_index,
                                    offsets.public_key_offset,
                                    PUBLIC_KEY_LENGTH)
        if !public_key {
            return Error
        }

        message = get_data_slice(all_tx_data,
                                 offsets.message_instruction_index,
                                 offsets.message_offset,
                                 offsets.message_length)
        if !message {
            return Error
        }

        result = falcon512_verify(signature, public_key, message)
        if result != Success {
            return Error
        }
    }
    return Success
}

// This function is re-used across precompiles in accordance with SIMD-0152
fn get_data_slice(all_tx_data, instruction_index, offset, length) {
    if instruction_index == 0xFFFF {
        instruction_data = all_tx_data.data
    } else {
        if instruction_index >= num_instructions {
            return Error
        }
        instruction_data = all_tx_data.instruction_datas[instruction_index]
    }

    start = offset
    // Check for overflow before arithmetic
    end = checked_add(offset, length)
    if end == OVERFLOW || end > length(instruction_data) {
        return Error
    }

    return instruction_data[start..end]
}
```

### Compute Cost
(**Tentative**)

Falcon-512 verification is computationally more expensive than Ed25519
due to the polynomial arithmetic involved. Benchmarking MUST be performed
on representative hardware to determine appropriate compute costs.

Based on preliminary estimates from reference implementations:
- Falcon-512 verification: approximately 0.5-1ms on modern hardware
- This translates to approximately 15,000-30,000 CUs per verification

Final compute costs MUST be determined through benchmarking in accordance
with established Solana compute unit pricing conventions (33ns/CU).

The maximum number of Falcon signatures per transaction is 8, consistent
with the limit defined in SIMD-0152 for all precompiles.

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

### 2. Syscall Instead of Precompile

Similar to the discussion in SIMD-0048/0075, implementing as a syscall
would ease integration for developers by avoiding instruction
introspection. However, precompiles are the established pattern for
signature verification in Solana, and following this pattern ensures
consistency.

### 3. On-chain BPF Implementation

Implementing Falcon verification in BPF would consume excessive compute
units due to the complex polynomial arithmetic involved, making it
impractical without precompile support.

### 4. Hash Function Variants

EIP-8052 proposes two variants: one using SHAKE-256 (NIST-compliant) and
one using Keccak256 (EVM-optimized). For Solana, we recommend only the
NIST-compliant SHAKE-256 variant to:
- Maintain compliance with draft FIPS 206
- Avoid unnecessary complexity
- Ensure interoperability with other draft FIPS 206 implementations

## Impact

### For dApp Developers

- New precompile available for post-quantum signature verification
- Larger signature and public key sizes require adjustments to account
  data structures and transaction layouts
- SDK updates will be needed to support Falcon key generation and signing

### For Validators

- Additional precompile to implement and maintain
- Higher computational cost per Falcon signature verification
- No impact on existing transaction processing

### For Core Contributors

- Implementation of Falcon-512 verification following draft FIPS 206
- Integration with existing precompile infrastructure per SIMD-0152
- Development of comprehensive test suites for cross-client compatibility

## Security Considerations

### Algorithm Security

Falcon-512 targets NIST Security Level I, providing approximately 128
bits of classical security and resistance against known quantum attacks.
The security is based on the hardness of the NTRU problem and the Short
Integer Solution (SIS) problem over NTRU lattices.

### Implementation Security

1. **Constant-time implementation**: The verification algorithm SHOULD be
   implemented in constant time where feasible to prevent timing attacks,
   though verification is inherently less sensitive than signing.

2. **Input validation**: All inputs (signatures, public keys) MUST be
   thoroughly validated before processing to prevent malformed input
   attacks.

3. **Signature uniqueness**: Unlike ECDSA, Falcon signatures are
   inherently non-malleable due to the use of a random salt (nonce) in
   the signing process. Each signing operation produces a unique
   signature.

### Cross-Client Consistency

As with other precompiles, it is imperative that there is bit-level
reproducibility between implementations across different validator
clients. Any discrepancy could cause network forks.

Recommendations:
- Development of a comprehensive test suite including NIST test vectors
- Active communication between client teams during implementation
- Use of well-audited reference implementations where possible

### Migration Considerations

This precompile does NOT replace Ed25519 for transaction signing. It
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

6. **Draft specification**: FIPS 206 is not yet finalized. If the final
   specification differs from the current draft, this precompile may
   require updates before activation. The implementation SHOULD track
   the NIST standardization process and incorporate any changes from
   the final FIPS 206 specification.

## Backwards Compatibility

Transactions using the Falcon precompile instruction cannot be processed
on Solana versions that do not implement this feature. A feature gate
MUST be used to enable this feature when the majority of the cluster is
running the required version.

Transactions that do not use this feature are not impacted.

Existing Ed25519 signatures and other precompiles (secp256k1, secp256r1)
continue to function unchanged.

## Scope

This precompile provides **signature verification only**. Key generation
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

- [Draft FIPS 206: FN-DSA (Falcon)](https://csrc.nist.gov/pubs/fips/206/ipd) - NIST Post-Quantum Signature Standard (Initial Public Draft)
- [EIP-8052: Falcon Signature Precompile](https://eips.ethereum.org/EIPS/eip-8052)
- [SIMD-0152: Precompiles](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0152-precompiles.md)
- [SIMD-0075: Precompile for secp256r1](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0075-precompile-for-secp256r1-sigverify.md)
- [Falcon Reference Implementation](https://falcon-sign.info/)
- [NIST PQC Standardization](https://csrc.nist.gov/projects/post-quantum-cryptography)
