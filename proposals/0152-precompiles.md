---
simd: '0152'
title: Precompiles
authors:
  - Emanuele Cesena
category: Standard
type: Core
status: Accepted
created: 2024-06-03
feature: ed9tNscbWLYBooxWA7FE2B5KHWs8A6sxfY8EzezEcoo
development:
  - Anza - [implemented](https://github.com/anza-xyz/agave/pull/1876)
---

## Summary

Define a unified behavior for precompiles, and highlight a few minor
changes to the existing precompiles to minimize differences.

## Motivation

Precompiles are special native programs designed to verify additional
transaction signatures.
They run without the VM and without loading any account, and they can access
data from other instructions, within the same transaction.

At the time of writing, two precompiles exist to verify Ed25519 and
Ethereum-like Secp256k1 signatures, and another one is being proposed to
support Secp256r1 signatures for FIDO Passkeys.

Historically, the two precompiles were built at different times and
by different people, so naturally there are some subtle differences
in how they behave, especially in edge cases.

The main goal of this document is to provide a specification for how
a precompile should behave, remove differences and provide guidelines
for future proposals.

In addition, we highlight 3 minor changes to the existing precompiles
that will simplify their behavior and make it easier to develop
alternative validator clients.

## Alternatives Considered

Leave as is.

## New Terminology

n/a

## Detailed Design

We first propose a specification for precompiles. Then we propose changes
to the existing precompiles with respect to the specification.

### Precompile Specification

Precompiles are special native programs designed to verify additional
signatures. Each precompile consists of a single `verify` instruction.

Precompiles are executed right after transaction signature verification,
they run without the VM and without loading any account. From a cost
perspective, they're included in the transaction fee (each signature to
verify counts as a transaction signature), but don't require any compute units.

If a transaction contains more than 8 precompile signatures, it must fail.

```
MAX_ALLOWED_PRECOMPILE_SIGNATURES = 8
```

The precompile instruction `verify` accepts the following data:

```
struct PrecompileVerifyInstruction {
  num_signatures:  u8,                       // Number of signatures to verify
  padding:         u8,                       // Single byte padding
  offsets:         PrecompileOffsets[],      // Array of `num_signatures` offsets
  additionalData?: Bytes,                    // Optional additional data, e.g.
                                             // signatures included in the same
                                             // instruction
}

struct PrecompileOffsets {
  signature_offset: u16 LE,                  // Offset to signature (offset within
                                             // the specified instruction data)
  signature_instruction_index: u16 LE,       // Instruction index to signature
  public_key_offset: u16 LE,                 // Offset to public key
  public_key_instruction_index: u16 LE,      // Instruction index to  public key
  message_offset: u16 LE,                    // Offset to start of message data
  message_length: u16 LE,                    // Size of message data
  message_instruction_index: u16 LE,         // Instruction index to message
}
```

The behavior of the precompile instruction `verify` is as follow:

1. If instruction `data` is empty, return error.
2. The first byte of `data` is the number of signatures `num_signatures`.
3. If `num_signatures` is 0, return error.
4. Expect (enough bytes of `data` for) `num_signatures` instances of
   `PrecompileOffsets`.
5. For each signature:
   a. Read `offsets`: an instance of `PrecompileOffsets`
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
message can be referred to by multiple instances of `PrecompileOffsets`.

If the precompile `verify` function returns any error, the whole transaction
should fail. Therefore, the type of error is irrelevant and is left as an
implementation detail.

In pseudo-code:

```
fn verify() {
  if data_length == 0 {
    return Error
  }
  num_signatures = data[0]
  if num_signatures == 0 {
    return Error
  }
  if data_length < (2 + num_signatures * size_of_offsets) {
    return Error
  }

  all_tx_data = { data, instruction_datas }

  data_position = 2
  for i in 0..num_signatures {
    offsets = (PrecompileOffsets)
      data[data_position..data_position+size_of_offsets]
    data_position += size_of_offsets

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
                                public_key_length)
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
    result = sigverify(signature, public_key, message)
    if result != Success {
      return Error
    }
  }
  return Success
}

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

### Changes to `Ed25519SigVerify111111111111111111111111111`

**Summary.**

- Change #1. Replace sigverify function with Dalek `strict_verify()`,
  the same used for transactions sigverify.

- Change #2. Implement step 3 above: "If `num_signatures` is 0, return error."

**Context.**

In Solana, transactions use Ed25199 signatures, and are validated using
the so called **strict verify**.
Compared to "RFC verify", strict verify enforces extra checks against
(certain types of) malleability.

The Ed25199 precompile currently implements a non-strict verify, so with
Change #1 we'll make it compatible with the way Solana verifies signatures.

Moreover, the Ed25519 precompile accepts a payload of `[0, 0]` as valid
(for no good reason), so Change #2 will prevent this anomaly.

Finally, the Ed25519 precompile interleaves retrieving instruction data
and parsing data types such as signatures and public keys.
While this goes against this specification and creates unnecessary complexity
in the return error code, we recommend to NOT change the internal behavior
(as the return error code doesn't really matter).

**FAQ.**

- **Q: Why does the Ed25199 precompile currently use `verify` instead of
  `strict_verify`?**
  A: No good reason, it was built without noticing the difference.

- **Q: If we switch to `strict_verify`, will some of the existing signatures
  break verification?**
  A: All signatures created by a "regular" library, i.e. following RFC,
  pass both `verify` and `strict_verify`.
  Only carefully crafted signatures can pass `verify` and not `strict_verify`.
  So this won't break any honest use case.

- **Q: Why not leaving it as is?**
  A: The `verify` is not well specified. In fact, it's behavior is slightly
  different in the older version of Dalek that Solana currently uses, versus
  the latest version of the same library.
  Trying to replicate all the edge cases is different validators is an
  unnecessary effort, not worth the risk of exposing different behaviors.

### Changes to `KeccakSecp256k11111111111111111111111111111`

**Summary.**

- Change #3. Implement step 3 above: "If `num_signatures` is 0, return error."

**Context.**

The KeccakSecp256k1 precompile currently accepts an input of `[0]`, as
in "verify 0 signatures", which is a useless instruction.
With Change #3 we'll avoid this anomaly.

We note that the KeccakSecp256k1 precompile has a slightly different
struct for offset, with instruction indexes of a single byte (and, as
a result, no special value of `0xFFFF` to indicate the "current instruction").
This is for historical reasons, and since modifying it would break
existing users, we recommend to NOT change the existing behavior.

## Impact

Reduce the complexity of existing precompiles, to simplify building
different validator clients.

## Security Considerations

All 3 changes are straightforward and have no impact on security.

## Backwards Compatibility

All 3 changes require a feature gate.
