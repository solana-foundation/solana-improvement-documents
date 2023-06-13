---
simd: '0057'
title: Events
authors:
  - Maximilian Schneider (max@mango.markets)
  - Nicholas Clarke (nicholasgclarke@mango.markets)
category: Standard/Core
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
provides this service to their customer. So it is deemed unreliably by the
teams not operating their own dedicated RPC nodes. Another issue is that these
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
message.

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

Discriminator - a unique 8 byte hash of a method signature or data type

## Detailed Design

In order to log an event, the SVM runtime should adopt a new syscall:
```
#include <sol/emit.h>
void sol_emit_(/* r1 */ uint64_t len,
               /* r2 */ const char *message);
```

RPC users can then subscribe to events emitted with various filters:

```
conn.onEvents(
    callback: EventCallback,
    commitment?: Commitment,
    filters?: EventsFilter[]);
```

Each event emitted calls the callback, the context of the instruction that
emitted the event is passed, specifically program id, instruction data &
accounts accessed.

```
EventCallback: ((
    event: Buffer,
    programId: PublicKey,
    ixData: Buffer,
    accounts: AccountMeta[],
    slot: number) => void)
```

DataSizeFilter and MemcmpFilter can be used to filter for specific event
types. AccountMetaFilter allows to restrict on events related to specific
POA or EOA. SignatureFilter allows to restrict to events of a single tx.

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
    signer?: bool
}
SignatureFilter : {
    signature: TransactionSignature
}

```

TBD: query event history
TBD: geyser interface

This feature will mostly supserseed the existing logging functionality for the
purpose of indexing, but developers will still need to debugging a program with
a quick text message, so it would be best to have them co-exist for as long as
possible.

## Impact

This will greatly simplify indexing and tooling for most of the complex
programs on solana. It will help new developers with EVM experience to onboard
to Solana quicker and 3rd party indexing providers to provide better data
services due to a standard way to parse application events.

## Security Considerations

The CU limit for sol_emit_ needs to be carefully adjusted given the heightened
indexing requirements for the RPC node.


## Backwards Compatibility *(Optional)*

This feature will mostly supserseed the existing logging functionality for the
purpose of indexing, but developers will still need to debugging a program with
a quick text message, so it would be best to have them co-exist for as long as
possible.