---
simd: '0057'
title: Events
authors:
  - Maximilian Schneider (max@mango.markets)
  - Nicholas Clarke (nicholasgclarke@mango.markets)
category: Standard
type: Core
status: Review
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
of their programs. In theory all account updates can be streamed at line-rate
out of a validator, but clients often lack the bandwith to consume those.

Common use-cases for this would be: executing a market order and reporting the
filled price to the user. Collect all orders on a market and aggregate them to
hourly candles. Allow UIs to listen to L3 order updates instead of streaming
full account updates for a large orderbook account.

A new syscall exposed to the runtime should allow dapps to reliably record
semantic and materialized (not raw) historical data, which clients subscribe
to via RPC. Clients can filter the events subscribed to with various rules,
to only receive the updates that are needed for a consistent client experience.
For CLOBs UI bandwidth requirements would reduce from 4MB/s to below 300kB/s.


## Alternatives Considered

Anchor events are a very popular solution, they pass base64 encoded binary rust
structs as string to the `sol_log_` syscall. The issue with this process is
that the sol_log_ destination buffer is truncated on many RPCs for complex
transactions with a lot of logs. As the setting is an operator setting
some of them choose to have very large log buffers, but not every RPC operator
provides this service to their customers. It's actually important to ensure
that the RPCs feeding the bigtable historical transactions also had a large
buffer otherwise you will experience truncation fetching logs older than the
RPCs local history, even when you configured your own RPC with a large log
buffer. Another issue is that these logs are used to also record runtime
information (compute units consumed) and plaintext debug messages, hence become
very difficult to parse, which is why many developers avoid structured event
logging and just use formatted strings.

A lot of developers would prefer a reliable and universally compatible solution
so recently Anchor has added a new feature called cpi_events. This is a
convention on how to call your own program via CPI and encode a binary string
as instruction data. It can be indexed via the RPC Transaction response, as
innerInstruction data is always persisted. It is more reliable than sol_log_
but it effectively reduces runtime limits like maximum CPI call depth for the
actual application. It also causeses unnecessary overhead in replay as a new
execution context neeeds to be created, for something as trivial as a log
message. For the average program (1-2mb) that would be around 5000-9000 CU per
1kb log message.

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

### Runtime 

In order to log an event, the SVM runtime must implement a new syscall
`sol_emit_`. The runtime must charge 100 + 0.25 * len CU per invocation to
account for the potential cost of including a blake3 hash of all log messages
in consensus.

```
#include <sol/emit.h>
void sol_emit_(/* r1 */ const char *message,
               /* r2 */ uint64_t len);
```

The validator should preserve these events similar to transaction history
and deliver them to it's operators via RPC & Geyser interfaces. Although
those are technically not part of the SIMD specification process, it's
important to publish a recommended spec for 3rd party providers so that Solana
Dapps don't run into vendor lock-in issues.

The cost for this operation is derived from a possible future inclusion of a
hash of all events in consensus. So future light-clients can verify the
authenticity of events received from third party rpc providers.

Bank deltas are currently hashed via sha256, the sha256 syscall charges
85 + 0.5 CU / byte. blake3 a roughly 2x faster to compute hash, would be
sufficient as well, hence 100 + 0.25 * len CU would be a good approximation
in this case. CPI for event logging would be only "cheaper" when using >4k
event payloads but most events are <512b in length. This cost enforces
theoretical limits to the amount of logs per transaction & block:
Limit per 200k CU: 800kB
Limit per 48M CU: 200MB

### RPC-Client

Clients should be able subscribe to events emitted with various filters, but
the restriction to a single program id is always required to prevent spoofing
of events:

```
rpc.onEvents(
    programId: PublicKey,
    callback: EventCallback,
    commitment?: Commitment,
    filters?: EventsFilter[]);
```

Each event emitted calls the callback, the context of the instruction that
emitted the event is referenced through:

- signature is the transaction's first signature
- suid is a sequence that follows the natural order of events as emitted
during the execution of a single transaction starting from 0
- outerInstructionIndex & innerInstructionIndex identify the instruction
which emitted the event

This ensures that clients can easily perform the following workflows:

1. deduplicate events received from multiple validators by the compound unique
index formed through (slot, signature, suid)
2. fetch the remaining details like account keys and instruction data
efficiently using `getTransaction`

```
EventCallback: ((
    data: Buffer,
    instructionContext: {
        signature: TransactionSignature,
        suid: number,
        outerInstructionIndex: number,
        innerInstructionIndex?: number,
    }
    context: {
        slot: number
    }
) => void)
```

DataSizeFilter and MemcmpFilter can be used to filter for specific event types
using their discriminator.  SignatureFilter allows to restrict to events
emitted by an individual transaction, while waiting for it's confirmation.

```
EventsFilter: (DataSizeFilter|MemcmpFilter|SignatureFilter)
DataSizeFilter: {
    dataSize: number;
}
MemcmpFilter: {
    memcmp: {
        bytes: string;
        offset: number;
    };
}
SignatureFilter : {
    signature: TransactionSignature
}
```

Clients should be able to query signatures that emitted events with various
filters. Any query must be constrained to a single programId to allow for
efficient indexing. Options must allow to paginate the result so that response
size and latency for programs with a lot of confirmed transactions can be
controlled. Either before or after can be used. The signature and suid are
definining respecetively the beginning or end of the sequence which does not
include the corresponding event. When both before and after are used the
before option has priority, meaning the sequence starts one event before
the specified one and ends either when reaching the limit or with the event
after the specified one. The order of the returned event sequence is
always in inverse chronological order, regarding how transactions are placed
in entries and then in a block and more specifically sorted descending by the
compound key: slot, block_entry, transaction_index_in_entry, suid.

```
EventLookupFilter: (DataSizeFilter|MemcmpFilter)

rpc.getSignaturesForEvent(
    programId: PublicKey,
    filters?: EventLookupFilter[],
    options?: {
        commitment?: Commitment,
        minContextSlot?: number,
        limit?: number,
        before?: {
            signature: TransactionSignature,
            suid: number,
        },
        after?: {
            signature: TransactionSignature,
            suid: number,
        },
    }
): Promise<TransactionSignature[]>
```

In addition clients should be able to query events directly and index them
reliably, without having to resolve signatures independently. Due to block
re-orgs it is recommened to populate the after option with the signature and
suid of the last known finalized event. Alternatively, the commitment level of
the whole response can be restricted to finalized.

```
rpc.getEvents(
    programId: PublicKey,
    filters?: EventLookupFilter[],
    options?: {
        commitment?: Commitment,
        encoding: 'base64',
        minContextSlot?: number,
        limit?: number,
        before?: {
            signature: TransactionSignature,
            suid: number,
        },
        after?: {
            signature: TransactionSignature,
            suid: number,
        },
    }
): Promise<EventResponse[]>


EventResponse: {
    data: Buffer,
    instructionContext: {
        signature: TransactionSignature,
        suid: number,
        outerInstructionIndex: number,
        innerInstructionIndex?: number,
    }
    context: {
        slot: number,
        confirmationStatus: "processed" | "confirmed" | "finalized"
    }
}
```

For previewing transactions before signing clients should be able to receive
events when calling `simulateTransaction`. The response should be extended
to include an array of EventResults under the key "events".

```
EventResult: {
    data: [string, "base64" | "base64+zstd"],
    programId: string,
    outerInstructionIndex:  number,
    innerInstructionIndex?: number,
}
```


### Geyser client

Clients should be able receive all events emitted, as part of the transaction
notification:

```
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub message: Vec<u8>;
    pub suid: u32,
    pub outerInstructionIndex: u16,
    pub innerInstructionIndex: Option<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransactionStatusMeta {
    pub status: TransactionResult<()>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub inner_instructions: Option<Vec<InnerInstructions>>,
    pub log_messages: Option<Vec<String>>,
    pub pre_token_balances: Option<Vec<TransactionTokenBalance>>,
    pub post_token_balances: Option<Vec<TransactionTokenBalance>>,
    pub rewards: Option<Rewards>,
    pub loaded_addresses: LoadedAddresses,
    pub return_data: Option<TransactionReturnData>,
    pub compute_units_consumed: Option<u64>,
    pub events: Option<Vec<Event>>
}
```

Once the current token programs have been migrated to emit events like ERC20
the pre_ / post_token_balances in TransactionStatusMeta could be deprecated.

## Impact

This will greatly simplify indexing and tooling for most of the complex
programs on solana. It will help new developers with EVM experience to onboard
to Solana quicker and 3rd party indexing providers to provide better data
services due to a standard way to parse application events. Examples would be
wallets that can easily parse and display incoming token transfers.
Once sufficiently adopted, events should superseed all spl-token program
related custom RPC features, like token balance changes etc.

## Security Considerations

The CU limit for sol_emit_ needs to be carefully adjusted given the heightened
indexing requirements for the RPC node. Events should never be truncated and
fully persist.

## Backwards Compatibility *(Optional)*

This feature will mostly supserseed the existing logging functionality for the
purpose of indexing, but developers will still need to debugging a program with
a quick text message, so it would be best to have them co-exist for as long as
possible.