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

Add a block footer to Solana blocks and expose Footer fields in the
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

## Detailed Design

### Data Layout

Solana blocks are organized in abstraction layers not entirely unlike the
arrangement of a typical network packet (e.g. MAC -> IP -> TCP -> HTTP). At the
highest layer a block consists of some number (~100+) FEC sets. A single FEC
set contains a handful of shreds (~32). Once sufficient shreds are available
the raw block data is reconstructed and reinterpreted as an array of entry
batches. Entry batches do not cross shred boundaries.

This SIMD add the following footer at the end of the raw block data. This
puts it on the same abstraction layer as serialized entry batch data. Put
differently, the serialized footer will be appended after the last serialized
entry batch in the block.

```
           Block Footer Layout
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_footer_flag           (64 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| version                     (64 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| footer_length               (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_producer_time_nanos   (64 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_user_agent_len         (8 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_user_agent        (0-255 bytes) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

Note that footer fields are packed together without any alignment requirements
or padding.
```

- `block_footer_flag: u64` will always be zero. The first 8 bytes of an entry
batch are always a positive number (the number of entries in the batch), so
this flag allows parsers to differentiate the footer from a normal entry batch.
This facilitates parsing block data and would also allow us to make the footer
optional if that's ever needed.

- `version: u64` is a positive integer which changes anytime a change is made to
the footer. The initial version will be 1.

- `footer_length: u16` is the length of the rest of the footer in bytes (i.e.
not including the `block_footer_flag`, `version`, and `footer_length` fields).

- `block_producer_time_nanos: u64` is a nanosecond UNIX timestamp representing
the time when the block producer started constructing the block.
"started constructing" is the point at which, from the perspective of the
leader, all of the consensus checks required for assuming leadership have
"just passed". For example, in Agave's pre-alpenglow implementations, this would
be in replay/maybe_start_leader. In a post-Alpenglow implementation, this would
be after receiving the proper vote/skip certificate for the previous slot.

- `block_user_agent_len: u8` the length of the `block_user_agent` string in
bytes.

- `block_user_agent: String` is a variable length utf-8 encoded string that
provides identifying information about the block producer.

Any other fields that are deemed necessary in the future may be added with a
corresponding change to `version` / `footer_length`. For example, SIMD
[0298](https://github.com/solana-foundation/solana-improvement-documents/pull/298)
proposes a header field, which could be added as a subsequent SIMD (or even
folded into this one).

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
agave/v2.2.15 (jito; doublezero; some-mod/v1.2.3)
```

Sometimes there may be software that coexists or runs alongside a validator
client. For example, current client development aims to make the transaction
scheduler modular, which would allow the transaction scheduler to be developed
independently from the client codebase. Validator clients that use
complementary software like this should add additional
`<product>/<product-version> <comment>` entries in the user agent string.

For example:

```
agave/v3.0.0 (doublezero) greedy-scheduler/v3 (mode:perf; another-flag)
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
        "blockUserAgent": "agave/v3.0.0 (doublezero) greedy-scheduler/v3 (mode:perf; another-flag)",
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

### Mandating the block footer

While it is possible to make the block footer optional thanks to the
`block_footer_flag` field, this proposal makes it mandatory. Blocks that don't
include a valid footer in the block payload will be flagged as dead blocks and
skipped by the other nodes in the cluster.

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