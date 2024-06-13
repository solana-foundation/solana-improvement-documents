---
simd: '0153'
title: ZK ElGamal Proof Program
authors:
  - Sam Kim
category: Standard
type: Core
status: Draft
created: 2024-06-13T00:00:00.000Z
feature: null
supersedes: null
superseded-by: null
extends: null
---
## Summary

Deprecate the existing ZK Token Proof program that is tailored for the SPL
Token program. Replace it with a more general zero-knowledge proof program
called the ZK ElGamal Proof program that is application independent.

## Motivation

The existing native ZK Token Proof program contains logic that is
quite specialized for the SPL Token program. For example, the program contains
instructions `VerifyTransfer`, `VerifyTransferWithFee`, and `VerifyWithdraw`
that verify zero-knowledge proofs that are tailor made for `Transfer` and
`Withdraw` instructions in the SPL Token confidential transfer extension.

The ZK Token Proof program is a native built-in program that is part of the
validator client. These type of programs should generally not favor any specific
application/program like the SPL Token program. Furthermore the logic
contained in the ZK Token Proof program enables private transfer of SPL tokens.
The SPL Token program is limited to confidential transfers (not
anonymous) and it contains an audit feature which enables authorities to decrypt
any confidential transfers. Nevertheless, including these type of privacy related
proof verification logic in the validator client can provide unnecessary legal
burden to the maintainers of the Solana validator clients.

In this document, we propose that we deprecate the existing ZK Token Proof
program and replace it with a more general ZK ElGamal Proof program that is
application independent. The new ZK ElGamal Proof program inherits parts of the
ZK Token Proof program that is independent of any specific application like
the logic to verify the validity of a public key or range of the encrypted in an
ElGamal ciphertext. It leaves out parts of the logic that is specific
to the SPL Token application like the logic to verify a zero-knowledge proof
required for a token transfer instruction.

## Alternatives Considered

We can activate the ZK Token Proof program. However, as explained above, this is
too specific to a particular application and also provides unnecessary burden to
Solana validator clients as the program contains privacy related logic.

We can deprecate the ZK Token Proof program. However, the program still contains
useful zero-knowledge proofs that are very general and will benefit many
projects in the ecosystem.

We can rewrite/compile the logic in the ZK Token Proof program as BPF
instructions, so that the program is independent of any validator client.
Unfortunately, the logic in the proof program are just too expensive to be
currently run inside the vm. This could change in the future; however,
launching the ZK Token Proof (or the new ZK ElGamal Proof) is of high priority
to the ecosystem as there are currently a number of projects that are waiting for
the ZK Token Proof program to be activated.

## New Terminology

n/a

## Detailed Design

The new ZK ElGamal Proof program contains a strict subset of the proof
verification instructions in the original ZK Token Proof program. We list the
current list of the instructions in the ZK Token Proof program and denote
whether they are included, removed, or renamed in the new ZK ElGamal Proof
program.

```rust
pub enum ProofInstruction {
    /// Close a zero-knowledge proof context state.
    ///
    /// This instruction will be left unchanged.
    CloseContextState,

    /// Verify a zero-balance proof.
    ///
    /// A zero-balance proof certifies that an ElGamal ciphertext encrypts the
    /// value zero.
    ///
    /// This instruction will be RENAMED to `VerifyZeroCiphertext`.
    VerifyZeroBalance,

    /// Verify a withdraw zero-knowledge proof.
    ///
    /// This instruction verifies zero-knowledge proofs that is necessary for
    /// the `Withdraw` instruction in SPL Token.
    ///
    /// This instruction will be REMOVED.
    VerifyWithdraw,

    /// Verify a ciphertext-ciphertext equality proof.
    ///
    /// A ciphertext-ciphertext equality proof certifies that two ElGamal
    /// ciphertexts encrypt the same message.
    ///
    /// This instruction will be left unchanged.
    VerifyCiphertextCiphertextEquality,

    /// Verify a transfer zero-knowledge proof.
    ///
    /// This instruction verifies zero-knowledge proofs that is necessary for
    /// the `Transfer` instruction in SPL Token.
    ///
    /// This instruction will be REMOVED.
    VerifyTransfer,

    /// Verify a transfer with fee zero-knowledge proof.
    ///
    /// This instruction verifies zero-knowledge proofs that is necessary for
    /// the `Transfer` instruction in SPL Token.
    ///
    /// This instruction will be REMOVED.
    VerifyTransferWithFee,

    /// Verify a public key validity zero-knowledge proof.
    ///
    /// A public key validity proof certifies that an ElGamal public key is
    /// well-formed and the prover knows the corresponding secret key.
    ///
    /// This instruction will be left unchanged.
    VerifyPubkeyValidity,

    /// Verify a 64-bit range proof.
    ///
    /// A range proof is defined with respect to a Pedersen commitment. The
    /// 64-bit range proof certifies that a Pedersen commitment holds an
    /// unsigned 64-bit number.
    ///
    /// This instruction is not specific to the SPL Token program, but since it
    /// can be subsumed by the `VerifyBatchRangeProofU64` instruction below, it
    /// will be REMOVED.
    VerifyRangeProofU64,

    /// Verify a 64-bit batched range proof.
    ///
    /// This instruction will be left unchanged.
    VerifyBatchedRangeProofU64,

    /// Verify 128-bit batched range proof.
    ///
    /// This instruction will be left unchanged.
    VerifyBatchedRangeProofU128,

    /// Verify 256-bit batched range proof.
    ///
    /// This instruction will be left unchanged.
    VerifyBatchedRangeProofU256,

    /// Verify a ciphertext-commitment equality proof.
    ///
    /// A ciphertext-commitment equality proof certifies that an ElGamal
    /// ciphertext and a Pedersen commitment encrypt/encode the same message.
    ///
    /// This instruction will be left unchanged.
    VerifyCiphertextCommitmentEquality,

    /// Verify a grouped-ciphertext with 2 handles validity proof.
    ///
    /// A grouped-ciphertext validity proof certifies that a grouped ElGamal
    /// ciphertext is well-defined, i.e. the ciphertext can be decrypted by
    /// private keys associated with its decryption handles.
    ///
    /// This instruction will be left unchanged.
    VerifyGroupedCiphertext2HandlesValidity,

    /// Verify a batched grouped-ciphertext with 2 handles validity proof.
    ///
    /// A grouped-ciphertext validity proof certifies that a grouped ElGamal
    /// ciphertext is well-defined, i.e. the ciphertext can be decrypted by
    /// private keys associated with its decryption handles.
    ///
    /// This instruction will be left unchanged.
    VerifyBatchedGroupedCiphertext2HandlesValidity,

    /// Verify a fee sigma proof.
    ///
    /// A `VerifyFeeSigma` proof certifies that a tuple of Pedersen commitments
    /// satisfy a percentage relation.
    ///
    /// This instruction will be RENAMED to `VerifyPercentageWithCap`.
    VerifyFeeSigma,

    /// Verify a grouped-ciphertext with 3 handles validity proof.
    ///
    /// A grouped-ciphertext validity proof certifies that a grouped ElGamal
    /// ciphertext is well-defined, i.e. the ciphertext can be decrypted by
    /// private keys associated with its decryption handles.
    ///
    /// This instruction will be left unchanged.
    VerifyGroupedCiphertext3HandlesValidity,

    /// Verify a batched grouped-ciphertext with 3 handles validity proof.
    ///
    /// A grouped-ciphertext validity proof certifies that a grouped ElGamal
    /// ciphertext is well-defined, i.e. the ciphertext can be decrypted by
    /// private keys associated with its decryption handles.
    ///
    /// This instruction will be left unchanged.
    VerifyBatchedGroupedCiphertext3HandlesValidity,
}
```

The implementation of the instructions that are not removed will remain the
same.

Instead of the modifying the existing ZK Token Proof program into ZK ElGamal
Proof program, we propose to simply remove the ZK Token Proof program altogether
and add a new built-in ZK ElGamal Proof program. The existing ZK Token Proof
program is not yet activated on any of the clusters.

## Impact

The existing ZK Token Proof program will be deprecated and removed. A new ZK
ElGamal Proof program will be added to the list of built-in programs.

## Security Considerations

The original ZK Token Proof program have been audited by multiple third party
auditing firms. Since the new ZK ElGamal Proof program will inherit the same
logic from the ZK Token Proof program, we do not expect additional security
vulnerabilities introduced with the new ZK ElGamal Proof program.

## Backwards Compatibility

The original ZK Token Proof program has not yet been activated on any of the
clusters. Therefore, deprecating it will simply be removing the unnecessary
logic and the feature gate.

The new ZK ElGamal program will require a new feature gate to be activated and
included as part of the list of native built-in programs.
