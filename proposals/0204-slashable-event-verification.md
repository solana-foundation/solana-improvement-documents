---
simd: '0204'
title: Slashable event verification
authors:
  - Ashwin Sekar
category: Standard
type: Core
status: Review
created: 2024-11-26
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal describes an enshrined on-chain program to verify proofs that a
validator committed a slashable infraction. This program creates reports on chain
for use in future SIMDs.

**This proposal does not modify any stakes or rewards, the program will
only verify and log infractions.**

## Motivation

There exists a class of protocol violations that are difficult to detect synchronously,
but are simple to detect after the fact. In order to penalize violators we provide
a means to record these violations on chain.

This also serves as a starting point for observability and discussions around the
economics of penalizing these violators. This is a necessary step to implement
slashing in the Solana Protocol.

## New Terminology

None

### Feature flags

`create_slashing_program`:

- `sProgVaNWkYdP2eTRAy1CPrgb3b9p8yXCASrPEqo6VJ`

## Detailed Design

On the epoch boundary where the `create_slashing_program` feature flag is first
activated the following behavior will be executed in the first block for the new
epoch:

1. Create a new program account at `S1ashing11111111111111111111111111111111111`
  with an upgrade authority set to the system program
  `11111111111111111111111111111111`

2. Verify that the program account
  `8sT74BE7sanh4iT84EyVUL8b77cVruLHXGjvTyJ4GwCe` has a verified build hash of
  `<FILL IN AFTER IMPLEMENTATION>` [\[1\]](#notes)

3. Copy the contents of `8sT74BE7sanh4iT84EyVUL8b77cVruLHXGjvTyJ4GwCe` into
  `S1ashing11111111111111111111111111111111111`

This program (hereafter referred to as the slashing program) supports 2
instructions `DuplicateBlockProof`, and `CloseProofReport`.

`DuplicateBlockProof` requires 1 account:

0. `proof_account`, expected to be previously intiialized with the proof data.

`DuplicateBlockProof` has an instruction data of 48 bytes, containing:

- `0x00`, a fixed-value byte acting as the instruction discriminator
- `offset`, an unaligned eight-byte little-endian unsigned integer indicating
  the offset from which to read the proof
- `slot`, an unaligned eight-byte little-endian unsigned integer indicating the
  slot in which the violation occured
- `node_pubkey`, an unaligned 32 byte array representing the public key of the
  node which committed the violation

We expect the contents of the `proof_account` when read from `offset` to
deserialize to a struct of two byte arrays representing the duplicate shreds.
The first 4 bytes correspond to the length of the first shred, and the 4 bytes
after that shred correspond to the length of the second shred.

```rust
struct DuplicateBlockProofData {
  shred1_length: u32,
  shred1: &[u8],
  shred2_length: u32,
  shred2: &[u8]
}
```

`DuplicateBlockProof` aborts if:

- The difference between the current slot and `slot` is greater than 1 epoch's
  worth of slots as reported by the `Clock` sysvar
- `offset` is larger than the length of `proof_account`
- `proof_account[offset..]` does not deserialize cleanly to a
  `DuplicateBlockProofData`.
- The resulting shreds do not adhere to the Solana shred format [\[2\]](#notes)
  or are legacy shred variants.
- The resulting shreds specify a slot that is different from `slot`.
- The resulting shreds specify different shred versions.

After deserialization the slashing program will attempt to verify the proof, by
checking that `shred1` and `shred2` constitute a valid duplicate proof for
`slot` and are correctly signed by `node_pubkey`. This is similar to logic used
in Solana's gossip protocol to verify duplicate proofs for use in fork choice.

### Proof verification

`shred1` and `shred2` constitute a valid duplicate proof if any of the following
conditions are met:

- Both shreds specify the same index and shred type, however their payloads
  differ
- Both shreds specify the same FEC set, however their merkle roots differ
- Both shreds specify the same FEC set and are coding shreds, however their
  erasure configs conflict
- At least one shred is a coding shred, and its erasure meta indicates an FEC set
  overlap.
- The shreds are data shreds with different indices and the shred with the lower
  index has the `LAST_SHRED_IN_SLOT` flag set

Note: We do not verify that `node_pubkey` was the leader for `slot`. Any node that
willingly signs duplicate shreds for a slot that they are not a leader for is
eligible for slashing.

---

### Signature verification

In order to verify that `shred1` and `shred2` were correctly signed by
`node_pubkey` we use instruction retrospection.

Using the `Instructions` sysvar we verify that the previous two instructions of
this transaction are for the program ID
`Ed25519SigVerify111111111111111111111111111`

For each of these instructions, verify the instruction data:

- The first byte is `0x01`
- The second byte (padding) is `0x00`

And then deserialize the remaining instruction data as 2 byte little-endian
unsigned integers:

```rust
struct Ed25519SignatureOffsets {
    signature_offset: u16,             // offset to ed25519 signature of 64 bytes
    signature_instruction_index: u16,  // instruction index to find signature
    public_key_offset: u16,            // offset to public key of 32 bytes
    public_key_instruction_index: u16, // instruction index to find public key
    message_data_offset: u16,          // offset to start of message data
    message_data_size: u16,            // size of message data
    message_instruction_index: u16,    // index of instruction data to get message
                                       // data
}
```

We wish to verify that these instructions correspond to

```
verify(pubkey = node_pubkey, message = shred1.merkle_root, signature = shred1.signature)
verify(pubkey = node_pubkey, message = shred2.merkle_root, signature = shred2.signature)
```

We use the deserialized offsets to calculate [\[3\]](#notes) the `pubkey`,
`message`, and `signature` of each instruction and verify that they correspond
to the `node_pubkey`, `merkle_root`, and `signature` specified by the shred payload.

If both proof and signer verification succeed, we continue on to store the incident.

---

### Incident reporting

After verifying a successful proof we store the results in a program derived
address for future use. The PDA is derived using the `node_pubkey`, `slot`, and
the violation type:

```rust
let (pda, _) = find_program_address(&[
  node_pubkey.to_bytes(),
  slot.to_le_bytes(),
  ViolationType::DuplicateBlock.to_u8(),
])
```

At the moment `DuplicateBlock` is the only violation type but future work will
add additional slashing types.

If the `pda` account has non-zero lamports, then we abort as the violation has
already been reported. Otherwise we create the account, with the slashing program
as the owner. In this account we store the following:

```rust
struct ProofReport {
  reporter: Pubkey,              // Fee payer, to allow the account to be closed
  epoch: Epoch,                  // Epoch in which this report was created
  pubkey: Pubkey,                // The pubkey of the node that committed the violation
  slot: Slot,                    // Slot in which the violation occured
  violation_type: u8,            // The violation type
  proof: Vec<u8>                 // The serialized proof
  proof_account: Option<Pubkey>, // Optional account where proof is stored instead
}
```

The `DuplicateBlockProofData` is serialized into the `proof` field. This provides
an on chain trail of the reporting process, since the `proof_account` supplied in
the `DuplicateBlockProof` account could later be modified.

The `pubkey` is populated with the `node_pubkey`. For future violation types that
involve votes, this will instead be populated with the vote account's pubkey.
The work in SIMD-0180 will allow the `node_pubkey` to be translated to a vote account
if needed.

Note that PDA's can only be created with a 10kb initial size.
Although not a problem for `DuplicateBlockProofData`, if future proof types require
more space, we allow the proof to be stored in a separate account, and linked back
to the PDA using the `proof_account` field.

---

### Closing the incident report

After the slashing violation has been processed by the runtime, the initial fee
payer may wish to close their `ProofReport` account to reclaim the lamports.

They can accomplish this via the `CloseProofReport` instruction which requires
2 accounts:

0. `report_account`: The PDA account storing the report: Writable, owned by the
  slashing program
1. `destination`: Writable account to reclaim the lamports

`CloseProofReport` has an instruction data of 42 bytes, containing:

- `0x01`, a fixed-value byte acting as the instruction discriminator
- `violation_type`, a one byte value acting as the violation type discriminator
- `slot`, an unaligned eight-byte little-endian unsigned integer indicating the
  slot which was reported
- `pubkey`, an unaligned 32 byte array representing the public key of the node
  which was reported

We abort if:

- `violation_type` is not `0x00` (corresponds to `DuplicateBlock` violation)
- Deriving the pda using `pubkey`, `slot`, and `ViolationType::DuplicateBlock`
  as outlined above does not result in the adddress of `report_account`
- `report_account` is not writeable
- `report_account` does not deserialize cleanly to `ProofReport`
- `report_account.reporter` is not a signer
- `report_account.epoch + 3` is greater than the current epoch reported from
  the `Clock` sysvar. We want to ensure that these accounts do not get closed before
  they are observed by indexers and dashboards.

Otherwise we close the `report_account` and credit the `lamports` to `destination`

---

## Alternatives Considered

This proposal deploys the slashing program in an "enshrined" account, only upgradeable
through code changes in the validator software. Alternatively we could follow the
SPL program convention and deploy to an account upgradeable by a multisig. This
allows for more flexibility in the case of deploying hotfixes or rapid changes,
however allowing upgrade access to such a sensitive part of the system via a handful
of engineers poses a security risk.

## Impact

A new program will be enshrined at `S1ashing11111111111111111111111111111111111`.

Reports stored in PDAs of this program might be queried for dashboards which could
incur additional indexing overhead for RPC providers.

## Security Considerations

None

## Drawbacks

None

## Backwards Compatibility

The feature is not backwards compatible

## Notes

\[1\]: Sha256 of program data, see
  https://github.com/Ellipsis-Labs/solana-verifiable-build/blob/214ba849946be0f7ec6a13d860f43afe125beea3/src/main.rs#L331
  for details.

\[2\]: The slashing program will support any combination of merkle shreds, chained
  merkle shreds, and retransmitter signed chained merkle shreds, see https://github.com/anza-xyz/agave/blob/4e7f7f76f453e126b171c800bbaca2cb28637535/ledger/src/shred.rs#L6
  for the full specification.

\[3\]: Example of offset calculation can be found here https://docs.solanalabs.com/runtime/programs#ed25519-program
