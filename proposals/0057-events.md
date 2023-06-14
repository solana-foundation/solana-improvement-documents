---
simd: '0057'
title: Events
authors:
  - Maximilian Schneider (max@mango.markets)
  - Nicholas Clarke (nicholasgclarke@mango.markets)
category: Standard
type: Core
status: Draft
created: 2023-06-13
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Propose a new event logging mechanism that is reliable and efficient.
Browser clients should be able to receive emitted events with minimal latency.
3rd party indexers should allow to horizontally scale access to historic
events.

## Motivation

Application developers need an easy way to reliably track the on-chain state
of their programs. Common use-cases for this would be: executing a market order
and reporting the filled price to the user. Collect all orders on a market and
aggregate them to an hourly candles. A specialized API exposed to the runtime
and RPC would allow dapps to reliably record historical data as well as RPC
providers to serve it effectively to any web client.

## Alternatives Considered

Anchor events are a very popular solution, they pass base64 encoded binary rust
structs as string to the `sol_log_` syscall. The issue with this process is
that the sol_log_ destination buffer is truncated on many RPCs for complex
transactions with a lot of logs. As the setting is an operator setting
some of them choose to have very large log buffers, but not every RPC operator
provides this service to their customers. There is a further issue with RPCs
and log truncation in that you also need to ensure that the RPCs feeding the
bigtable historical transactions also had a large log buffer otherwise you can
experience truncation fetching logs older than the RPCs local history even when
you're pointed at an RPC with a large log buffer. Another issue is that these
logs are used to also record runtime information (compute units consumed) and
plaintext debug message, hence become very difficult to parse, which is why
many developers avoid structured event logging via anchor and just use
formatted strings.

A lot of developers would prefer a reliable and universally compatible solution
so recently Anchor has added a new feature called cpi_events. This is a
convention on how to call your own program via CPI and encode a binary string
as instruction data. This can be indexed via the RPC Transaction response, as
innerInstruction data is always persisted. It is more reliable than sol_log_
but it effectively reduces runtime limits like maximum CPI call depth for the
actual application. It also causeses unnecessary overhead in replay as a new
execution context neeeds to be created, for something as trivial as a log
message. There is a potential security risk in the this system in that external
programs could spoof anchor events (and the program emitting them) with CPI
as anchor event parsing is completely text based.

In comparison to ethereum and cosmos, solana's preferred access for transaction
meta-data is via address filtering, e.g. `getSignaturesForAddress` call.

1. Topics: the ethereum yellow paper instead specifies that every event can be
attributed to up to 4 topics. Each topic is a 32 byte subscription key, users
can use to filter these events via RPC.

This seems restrictive compared to the expressivenes that is guaranteed through
the context of each instruction, program id and account metas, this data is
anyways available at the RPC node.

2. Schema: the cosmos sdk requires every event to have a protobuf schema, this
makes it very easy for third parties to come in and build generalized indexers
for the cosmos ecosystem. It also allows to specify complex filter queries that
can evaluated on the RPC node to stream or retrieve events.

Parsing the event payload is not a task for the rpc layer, and should instead
be handled by 3rd party providers. As schemas on solana are flexible and can
change with program updates, this would require access to historic schema
definitions / account state.

## New Terminology

Discriminator - a unique n byte sequence often a 8 byte hash of a method
signature or data type, that should be used to tag the type of a data structure
and encoded always in the very first bytes of an event, account or instruction
for the sake of efficient parsing.

## Detailed Design

In order to log an event, the SVM runtime must implement a new syscall:

```
#include <sol/emit.h>
void sol_emit_(/* r1 */ uint64_t len,
               /* r2 */ const char *message);
```

RPC must allow users to subscribe to events emitted with various filters:

```
conn.onEvents(
    callback: EventCallback,
    commitment?: Commitment,
    filters?: EventsFilter[]);
```

Each event emitted calls the callback, the context of the instruction that
emitted the event is referenced through Transaction signature and indexes that
uniquely identify the instruction. `getTransaction` can be used to fetch the
remaining details like account keys and instruction data efficiently in case
they are needed for indexing.

```
EventCallback: ((
    event: Buffer,
    instructionContext: {
        transactionSignature: TransactionSignature,
        outerInstructionIndex: number,
        innerInstructionIndex?: number,
    }
    context: {
        eventIndexInBlock: number,
        slot: number
    }
) => void)
```

DataSizeFilter and MemcmpFilter can be used to filter for specific event types
using their discriminator. AccountMetaFilter allows to restrict on events
emitted from transactions reading or writing from/to specific POA & EOA.
Similarly it allows to restricted based on specific EOA signers as well on
executing certain programs.
SignatureFilter allows to restrict to events emitted by an individual
transaction, while waiting for it's confirmation.

```
EventsFilter: (DataSizeFilter|MemcmpFilter|AccountMetaFilter|SignatureFilter)
DataSizeFilter: {
    dataSize: number;
}

MemcmpFilter: {
    memcmp: {
        bytes: string;
        offset: number;
    };
}
AccountMetaFilter : {
    pubkey: Pubkey,
    writeable?: bool,
    signer?: bool,
    program? bool,
}
SignatureFilter : {
    signature: TransactionSignature
}
```


## TODO

1. implementation: query event history
1. implementation: geyser interface
1. decide on:
POA signers are currently not exposed via RPC, AccountMetaFilters could expose
them accidentally. The signer flag could be limited on purpose to be used with
EOA to prevent access. Or alternatively the signer information could be added
to the Inner Instructions Structure.


## Impact

This will greatly simplify indexing and tooling for most of the complex
programs on solana. It will help new developers with EVM experience to onboard
to Solana quicker and 3rd party indexing providers to provide better data
services due to a standard way to parse application events. Examples would be
wallets that can easily parse and display user transaction history.
Once sufficiently adopted, events should superseed all spl-token program
related custom rpc features, like token balance changes etc.

## Security Considerations

The CU limit for sol_emit_ needs to be carefully adjusted given the heightened
indexing requirements for the RPC node. Events should never be truncated and
fully persist.


## Backwards Compatibility *(Optional)*

This feature will mostly supserseed the existing logging functionality for the
purpose of indexing, but developers will still need to debugging a program with
a quick text message, so it would be best to have them co-exist for as long as
possible.