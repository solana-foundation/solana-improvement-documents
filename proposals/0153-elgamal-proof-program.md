---
simd: '0153'
title: ZK ElGamal Proof Program
authors:
  - Sam Kim
category: Standard
type: Core
status: Accepted
created: 2024-06-13
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
validator client. These types of programs should generally not favor any specific
application/program like the SPL Token program.

In this document, we propose that we deprecate the existing ZK Token Proof
program and replace it with a more general ZK ElGamal Proof program that is
application independent. The new ZK ElGamal Proof program inherits parts of the
ZK Token Proof program that is independent of any specific application like
the logic to verify the validity of a public key or range of the encrypted in an
ElGamal ciphertext. It leaves out parts of the logic that are specific
to the SPL Token application like the logic to verify a zero-knowledge proof
required for a token transfer instruction.

## Alternatives Considered

We can activate the ZK Token Proof program. However, as explained above, this is
too specific to a particular application.

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
verification [instructions](https://github.com/anza-xyz/agave/blob/master/zk-token-sdk/src/zk_token_proof_instruction.rs#L48)
in the original ZK Token Proof program. We list the
the instructions in the ZK Token Proof program that are to be either renamed or
removed in the new ZK ElGamal Proof program.

The following set of instructions will be RENAMED:

- `VerifyZeroBalance`: Verifies a proof that certifies that an ElGamal
  ciphertext encrypts the value zero.

  This instruction will be RENAMED to `VerifyZeroCiphertext`.

- `VerifyFeeSigma`: Verifies a proof that certifies that a tuple of Pedersen
  commitments satisfy a percentage relation.

  This instruction will be RENAMED to `VerifyPercentageWithCap`.

The following set of instructions will be REMOVED:

- `VerifyWithdraw`: Verifies the zero-knowledge proofs that are necessary for the
  `Withdraw` instruction in SPL Token.

- `VerifyTransfer`: Verifies the zero-knowledge proofs that are necessary for
  the `Transfer` instruction in SPL Token.

- `VerifyTransferWithFee`: Verifies the zero-knowledge proofs that are necessary
  for the `Transfer` instruction in SPL Token.

- `VerifyRangeProofU64`: Verifies that a Pedersen commitment contains a positive
  64-bit value. This instruction is not specific to the SPL Token program, but
  it can be subsumed by the existing `VerifyBatchRangeProofU64` instruction.

The implementation of the instructions that are not removed will remain the
same.

Instead of modifying the existing ZK Token Proof program into the ZK ElGamal
Proof program, we propose to simply remove the ZK Token Proof program altogether
and add a new built-in ZK ElGamal Proof program. The existing ZK Token Proof
program is not yet activated on any of the clusters.

## Impact

The existing ZK Token Proof program in the address
`ZkTokenProof1111111111111111111111111111111` will be deprecated and removed. A
new ZK ElGamal Proof program will be added to the list of built-in programs in
the address `ZkE1Gama1Proof11111111111111111111111111111`.

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
