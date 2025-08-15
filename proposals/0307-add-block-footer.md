---
simd: '0307'
title: Add Block Footer
authors:
  - jherrera-jump (Firedancer)
category: Standard
type: Core
status: Review
created: 2025-06-17
feature: <pubkey>
development:
  - Anza - TBD
---

## Summary

Add a block footer to Solana blocks and expose footer fields in the
`getBlock` rpc endpoint.

## Motivation

For the purposes of historical monitoring, development, and auditing, it is
important to know exactly who produced a block and when it was produced.
Currently, this information can be partially inferred from Gossip and from vote
timestamps. Unfortunately there are some problems with the current approach:

- The information from gossip is ephemeral. Currently a peer needs to record
  and persist it. This may cause synchronization issues when matching client
  updates in gossip with the correct slot.
- Gossip lacks important information that may useful for monitoring (e.g.
  scheduler used, mods, configuration settings, etc).
- Vote timestamps have a granularity of 1-second, so they cannot be used to
  estimate block duration.
- Vote timestamps will be removed with Alpenglow.

This SIMD solves these issues by including relevant information in a static
block footer.

## New Terminology

No new terms, but the following definitions are given for clarity:

- Client - The software run by leaders to interface with a solana cluster.
  (e.g. `agave` or `frankendancer`)
- Block Producer - The client that produced a given block
- Scheduler - The system responsible for processing incoming transactions and
  ordering them for block construction.
- Forward Error Correction set (FEC set) - A collection of shreds. At a high
  level, this is a construct that leverages Reed-Solomon encoding to overcome
  the problem of data loss from packet drops.
- Shreds - A fixed chunk of encoded raw block data.
- Entry Batch - An array of entries.
- Entry - An array of transactions.
- Block Marker - A chunk of structured non-transaction data that can be placed
  before, after, or in-between entry batches in a block.

## Detailed Design

### Data Layout

Solana blocks are organized in abstraction layers not entirely unlike the
arrangement of a typical network packet (e.g. MAC -> IP -> TCP -> HTTP). At the
highest layer a block consists of some number (~100+) FEC sets. A single FEC set
contains a handful of shreds (~32). Once sufficient shreds are available the raw
block data is reconstructed and reinterpreted as an array of entry batches.
Entry batches are aligned with shred boundaries (i.e. they will start/stop at a
shred boundary).

This SIMD introduces the idea of a block marker. This is a piece of data that
would take the place of an entry batch in an incoming shred stream. Entry batch
data starts with an 8 byte value that represents the number of entries in the
batch. This number cannot be zero. By including 8 zero bytes at the beginning of
the block marker header, a replay parser can differentiate it from a regular
entry batch. A block marker chunk has the following versioned header:

```

        Block Marker Header Layout
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_marker_flag      (64 bits of 0) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| version=1                   (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| variant                      (8 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| length                      (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|              --payload--              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

Note that all data fields below are packed together without any alignment or
padding constraints.

```

`block_marker_flag: u64`: will always be zero. The first 8 bytes of an entry
batch are always a positive number (the number of entries in the batch), so this
flag allows parsers to differentiate the block marker from a normal entry batch.

- `version: u16` is a positive integer which changes anytime a change is made to
the block marker header. The initial version will be 1.

- `variant: u8` is an integer which identifies the structure of the block marker
payload. For example, the block footer will be identified by `variant=0`. New
metadata may be added without changing the footer by adding a new variant which
corresponds to a differently specified payload.

- `length: u16` is the length of the block marker payload in bytes (i.e. not
including the `block_marker_flag`, `version`, `variant`, and `length` fields).
Though not necessary, this will make it easier for block parsers to ignore
certain variants.

This header would precede any block marker variant inserted into the block data.
Although the block marker data length can be inferred from the version/variant,
an explicit length field is included for non-client parsers which discard
non-transactional data from the block. This ensures that we don't create a
dependency on these parsers when pushing out new/updated block markers. It also
greatly simplifies the implementation of hardware parsers, especially
considering that some variants contain variable length fields.

This SIMD also proposes the following block marker variant with an additional
constraint: it must occur once after the last entry batch in a block.  The block
footer is meant to contain general block and producer metadata, along with any
metrics that must be computed after the block has been produced.

```

     Block Marker Variant -- Footer
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_marker_flag      (64 bits of 0) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| version=1                   (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| variant=0                    (8 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| length                      (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| footer_version=1            (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_producer_time_nanos   (64 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_user_agent_len         (8 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_user_agent        (0-255 bytes) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

Note that all data fields below are packed together without any alignment or
padding constraints.

```

- `footer_version: u16` is a positive integer which changes anytime a change is
made to the footer. The initial version will be 1.

- `block_producer_time_nanos: u64` is a nanosecond UNIX timestamp representing
the time when the block producer started constructing the block. "started
constructing" is the point at which, from the perspective of the leader, all of
the consensus checks required for assuming leadership have "just passed". For
example, in Agave's pre-alpenglow implementations, this would be in
replay/maybe_start_leader. In a post-Alpenglow implementation, this would be
after receiving the proper vote/skip certificate for the previous slot.

- `block_user_agent_len: u8` the length of the `block_user_agent` string in
bytes.

- `block_user_agent: String` is a variable length utf-8 encoded string that
provides identifying information about the block producer.

Any other fields that are deemed necessary in the future may be added in one of
two ways.

- amend the `footer_version` and add the field the footer
- create a new block marker variant and add the field to its payload

### Footer Field Specification

Footer fields will be unilaterally populated by their respective block producer
without any enforced constraint on their contents. This SIMD includes the
following fields in the footer

- `block_producer_time_nanos`: u64
- `block_user_agent_len`: u8
- `block_user_agent`: String

Because it is desirable to maintain cluster-wide diagnostics this SIMD provides
a suggested format for the `block_user_agent` string which includes basic
information about the block producer. This should be a UTF-8 encoded variable
length string (up to 255 bytes long). It is not necessarily null-terminated. It
is expected that all producers use the format specified here, though this will
not be enforced. Clients are encouraged to at the very least use a valid utf-8
string and include extraneous data in a way that coheres with the specification.
The format is loosely based on HTTP `user-agent` header format specification:

```
<product>/<product-version> <comment>
```

The first entry will always be the software client.

```
client/client_version <client_details> 
```

Options for `client` currently include:

- `agave`
- `frankendancer`
- `firedancer`

`client_version` should be consistent with the information stored on-chain (in
`ConfigProgram`). Software forks (e.g. `jito-agave`) should put one of
the 3 base clients and can specify details about the fork in the comment.

The comment should be in parentheses and contain a semicolon separated
list of flags. A flag has an unrestricted format, but should represent a
feature that is contained and enabled in the client it describes.

e.g.

```
agave/v2.2.15 (jito; double0; some-mod/v1.2.3)
```

Sometimes there may be software that coexists or runs alongside a validator
client. For example, current client development aims to make the transaction
scheduler modular, which would allow the transaction scheduler to be developed
independently from the client codebase. Validator clients that use
complementary software like this should add additional
`<product>/<product-version> <comment>` entries in the user agent string.

For example:

```
agave/v3.0.0 (paladin; double0) greedy-scheduler/v3 (mode:perf; another-flag)
```

### RPC Protocol Changes

The `getBlock` RPC response will be extended to, optionally, include all footer
fields. The request will be extended with the `footer` parameter, which lets
the client signal that they want the footer fields in the response. By default,
footer fields will be included in the response.

Sample Request Payload

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "getBlock",
  "params": [
    378967388,
    {
      "encoding": "json",
      "maxSupportedTransactionVersion": 0,
      "transactionDetails": "full",
      "rewards": false,
      "footer": true
    }
  ]
}
```

Sample Response Payload

<!-- markdownlint-disable MD013 -->
```json
{
  "jsonrpc": "2.0",
  "result": {
    "blockHeight": 428,
    "blockTime": null,
    "blockhash": "3Eq21vXNB5s86c62bVuUfTeaMif1N2kUqRPBmGRJhyTA",
    "parentSlot": 429,
    "previousBlockhash": "mfcyqEXB3DnHXki6KjjmZck6YjmZLvpAByy2fj4nh6B",
    "footer": {
        "blockProducerTimeNanos": 1750176982899968023,
        "blockUserAgent": "agave/v3.0.0 (double0) greedy-scheduler/v3 (mode:perf; another-flag)",
    },
    "transactions": [
      {
        "meta": {
          "err": null,
          "fee": 5000,
          "innerInstructions": [],
          "logMessages": [],
          "postBalances": [499998932500, 26858640, 1, 1, 1],
          "postTokenBalances": [],
          "preBalances": [499998937500, 26858640, 1, 1, 1],
          "preTokenBalances": [],
          "rewards": null,
          "status": {
            "Ok": null
          }
        },
        "transaction": {
          "message": {
            "accountKeys": [
              "3UVYmECPPMZSCqWKfENfuoTv51fTDTWicX9xmBD2euKe",
              "AjozzgE83A3x1sHNUR64hfH7zaEBWeMaFuAN9kQgujrc",
              "SysvarS1otHashes111111111111111111111111111",
              "SysvarC1ock11111111111111111111111111111111",
              "Vote111111111111111111111111111111111111111"
            ],
            "footer": {
              "numReadonlySignedAccounts": 0,
              "numReadonlyUnsignedAccounts": 3,
              "numRequiredSignatures": 1
            },
            "instructions": [
              {
                "accounts": [1, 2, 3, 0],
                "data": "37u9WtQpcm6ULa3WRQHmj49EPs4if7o9f1jSRVZpm2dvihR9C8jY4NqEwXUbLwx15HBSNcP1",
                "programIdIndex": 4
              }
            ],
            "recentBlockhash": "mfcyqEXB3DnHXki6KjjmZck6YjmZLvpAByy2fj4nh6B"
          },
          "signatures": [
            "2nBhEBYYvfaAe16UMNqRHre4YNSskvuYgx3M6E4JP1oDYvZEJHvoPzyUidNgNX5r9sTyN1J9UxtbCXy2rqYcuyuv"
          ]
        }
      }
    ]
  },
  "id": 1
}
```
<!-- markdownlint-restore -->

### Mandating the footer

This proposal makes the block footer. Blocks that don't include a valid footer
in the block payload must be flagged as dead blocks and skipped by the other
nodes in the cluster.

## Alternatives Considered

- Do nothing
  - We can't estimate block time / duration with sufficient granularity. We
    won't be able to estimate at all when votes are changed in alpenglow.
  - We will continue to have an incomplete, ephemeral record of who produced
    blocks.
- derive timestamp footer field from consensus and enforce user agent format
  - This can and probably should be implemented as a future SIMD. Meanwhile,
    these fields are still useful since
    1. most of the cluster is expected to
    be honest, so monitoring tools may still use them for cluster-wide
    analytics and
    2. block producers still use these fields to self-monitor
    their performance.
- Send block producer information via gossip instead
  - The information is short-lived and depends on physical network availability
- Update this information in an on-chain account instead (e.g. ConfigProgram)
  - Same issue as above, the information is short-lived.

## Impact

This change will enable more reliable monitoring and benchmarking for operators
and for the community. Clients and indexers will need to extend both in-memory
and long-term block storage to be aware of the new columns added to the block
footer. The client rpc engine will need to change to support the new fields.

## Security Considerations

- The footer fields are untrusted and purely informational. Tools that expose
  these fields to external users should clearly communicate their untrusted
  nature.

## Drawbacks

- No expected drawbacks beyond minimal resource overhead.

## Backwards Compatibility

- RPC requests for old slots should properly document and return a suitable
  default value (e.g. None).
- Clients that don't implement this SIMD will reject new blocks because they
will fail to parse the new footer.
- Because this footer is mandatory, leaders that produce blocks without a
footer will skip.