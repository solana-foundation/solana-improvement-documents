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
  owned by the default upgradeable loader with an upgrade authority set to `None`,
  and create the associated program data account.

2. Verify that the buffer account
  `S1asHs4je6wPb2kWiHqNNdpNRiDaBEDQyfyCThhsrgv` has a verified build hash of
  `192ed727334abe822d5accba8b886e25f88b03c76973c2e7290cfb55b9e1115f` [\[1\]](#notes)

3. Serialize the contents of `S1asHs4je6wPb2kWiHqNNdpNRiDaBEDQyfyCThhsrgv` into
  the program data account for `S1ashing11111111111111111111111111111111111`

4. Invoke the loader to deploy the new program account and program data account.
  This step also updates the program cache.

4. Zero out the buffer account `S1asHs4je6wPb2kWiHqNNdpNRiDaBEDQyfyCThhsrgv`, and
  update the changes to capitalization and account data lengths accordingly.

This is the only protocol change that clients need to implement. The remaining
proposal describes the function of this program, hereafter referred to as the
slashing program.

The buffer account `S1asHs4je6wPb2kWiHqNNdpNRiDaBEDQyfyCThhsrgv` will hold the
v1.0.0 release of the slashing program
https://github.com/solana-program/slashing/releases/tag/v1.0.0

### Slashing Program

This slashing program supports two instructions `DuplicateBlockProof`, and
`CloseViolationReport`.

`DuplicateBlockProof` requires 2 accounts, the `Instructions` sysvar, and the
system program :

0. `proof_account`, expected to be previously initialized with the proof data.
1. `report_account`, the PDA in which to store the violation report. See the below
    section for details. Must be writable.
2. `instructions`, Instructions sysvar
3. `system_program_account`, required to create the violation report.

`DuplicateBlockProof` has an instruction data of 305 bytes, containing:

- `0x01`, a fixed-value byte acting as the instruction discriminator
- `offset`, an unaligned eight-byte little-endian unsigned integer indicating
  the offset from which to read the proof
- `slot`, an unaligned eight-byte little-endian unsigned integer indicating the
  slot in which the violation occured
- `node_pubkey`, an unaligned 32 byte array representing the public key of the
  node which committed the violation
- `reporter`, an unaligned 32 byte array representing the account to credit
  as the first reporter of this violation if a successful slashing report is
  written.
- `destination`, an unaligned 32 byte array representing the account to reclaim
  the lamports if a successful slashing report account is created and then later
  closed.
- `shred_1_merkle_root`, an unaligned 32 byte array representing the merkle root
  of the first shred in the `proof_account`
- `shred_1_signature`, an unaligned 64 byte array representing the signature
  of `node_pubkey` on the first shred in the `proof_account`
- `shred_2_merkle_root`, an unaligned 32 byte array representing the merkle root
  of the second shred in the `proof_account`
- `shred_2_signature`, an unaligned 64 byte array representing the signature
  of `node_pubkey` on the second shred in the `proof_account`

We expect the contents of the `proof_account` when read from `offset` to
deserialize to two byte arrays representing the duplicate shreds.
The first 4 bytes correspond to the length of the first shred, and the 4 bytes
after that shred correspond to the length of the second shred.

```rust
struct DuplicateBlockProofData {
  shred1_length: u32      // Unaligned four-byte little-endian unsigned integer,
  shred1: &[u8]           // `shred1_length` bytes representing a shred,
  shred2_length: u32      // Unaligned four-byte little-endian unsigned integer,
  shred2: &[u8]           // `shred2_length` bytes representing a shred,
}
```

Users are expected to populate the `proof_account` themselves, using an onchain
program such as the Record program.

`DuplicateBlockProof` aborts if:

- The difference between the current slot and `slot` is greater than 1 epoch's
  worth of slots as reported by the `Clock` sysvar
- The `destination` is equal to the address of `pda_account`
- `offset` is larger than the length of `proof_account`
- `proof_account[offset..]` does not deserialize cleanly to a
  `DuplicateBlockProofData`.
- The resulting shreds do not adhere to the Solana shred format [\[2\]](#notes)
  or are legacy shred variants.
- The resulting shreds specify a slot that is different from `slot`.
- The resulting shreds specify different shred versions.

After deserialization the slashing program will attempt to verify the proof, by
checking that `shred1` and `shred2` constitute a valid duplicate block proof for
`slot` and are correctly signed by `node_pubkey`. This is similar to logic used
in Solana's gossip protocol to verify duplicate block proofs for use in fork choice.

#### Proof verification

`shred1` and `shred2` constitute a valid duplicate block proof if any of the
following conditions are met:

- Both shreds specify the same index and shred type, however their payloads
  differ
- Both shreds specify the same FEC set, however their merkle roots differ
- Both shreds specify the same FEC set and are coding shreds, however their
  erasure configs conflict
- The shreds specify different FEC sets, the lower index shred is a coding shred,
  and its erasure meta indicates an FEC set overlap
- The shreds specify different FEC sets, the lower index shred has a merkle root
  that is not equal to the chained merkle root of the higher index shred
- The shreds are data shreds with different indices and the shred with the lower
  index has the `LAST_SHRED_IN_SLOT` flag set

Note: We do not verify that `node_pubkey` was the leader for `slot`. Any node that
willingly signs duplicate shreds for a slot that they are not a leader for is
eligible for slashing.

---

#### Signature verification

In order to verify that `shred1` and `shred2` were correctly signed by
`node_pubkey` we use instruction introspection.

Using the `Instructions` sysvar we verify that the previous instruction of
this transaction are for the program ID
`Ed25519SigVerify111111111111111111111111111`

For this instruction, verify the instruction data:

- The first byte is `0x02`
- The second byte (padding) is `0x00`

Verify that the remaining instruction data represents two signature offsets
which is specified as 2 byte little-endian unsigned integers:

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

The instruction indices must point to the `DuplicateBlockProof` instruction and
the offsets into the instruction data where these values are stored.

If both proof and signer verification succeed, we continue on to store the incident.

---

#### Incident reporting

After verifying a successful proof we store the results in a program derived
address for future use. The PDA is derived using the `node_pubkey`, `slot`, and
the violation type:

```rust
let (pda, _) = find_program_address(&[
  node_pubkey.to_bytes(),        // 32 byte array representing the public key
  slot.to_le_bytes(),            // Unsigned little-endian eight-byte integer
  1u8,                           // Byte representing the violation type
])
```

If the `pda` is not equal to the addres of the `pda_account` then we abort.

At the moment `DuplicateBlock` is the only violation type but future work will
add additional slashing types.

We expect the `pda` account to be prefund-ed by the user to contain enough lamports
to store a `ProofReport`.

If the `pda` account has any data, is owned by the slashing program, and the version
number is non zero then we abort as the violation has already been reported.
Otherwise we allocate space in the account, and assign the slashing program as
the owner. In this account we store the following:

```rust
struct ProofReport {
  version: u8,                     // 1 byte specifying the version number,
                                   // currently 1

  reporter: Pubkey,                // 32 byte array representing the pubkey of the
                                   // Fee payer, who reported this violation

  destination: Pubkey,             // 32 byte array representing the account to
                                   // credit the lamports when this proof report
                                   // is closed.

  epoch: Epoch,                    // Unaligned unsigned eight-byte little endian
                                   // integer representing the epoch in which this
                                   // report was created

  violator: Pubkey,                // 32 byte array representing the pubkey of the
                                   // node that committed the violation

  slot: Slot,                      // Unaligned unsigned eight-byte little endian
                                   // integer representing the slot in which the
                                   // violation occured

  violation_type: u8,              // Byte representing the violation type
}
```

immediately followed by a `DuplicateBlockProofData`.

This proof data provides an on chain trail of the reporting process, since the
`proof_account` supplied in the `DuplicateBlockProof` instruction could later
be modified.

The `pubkey` is populated with the `node_pubkey`. For future violation types that
involve votes, this will instead be populated with the vote account's pubkey.
The work in SIMD-0180 will allow the `node_pubkey` to be translated to a vote account
if needed.

---

#### Closing the incident report

In a future SIMD the reports will be used for runtime processing. This is out of
scope, but after this period has passed,  the initial fee payer may wish to close
their `ProofReport` account to reclaim the lamports.

They can accomplish this via the `CloseViolationReport` instruction which requires
two accounts

0. `report_account`: The PDA account storing the report: Writable, owned by the
  slashing program
1. `destination_account`: The destination account to receive the lamports: Writable

`CloseViolationReport` has an instruction data of one byte, containing:

- `0x00`, a fixed-value byte acting as the instruction discriminator

We abort if:

- `report_account` is not owned by the slashing program
- `report_account` does not deserialize cleanly to `ProofReport`
- `report_account.destination` does not match `destination_account`
- `report_account.epoch + 3` is greater than the current epoch reported from
  the `Clock` sysvar. We want to ensure that these accounts do not get closed before
  they are observed by indexers and dashboards.

The three epoch window is somewhat arbitrary, we only need the `report_account` to
last at least one epoch in order to for it to be observed by the runtime as part
of a future SIMD.

Otherwise we set the owner of `report_account` to the system program, rellocate
the account to 0 bytes, and credit the `lamports` to `destination_account`

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
