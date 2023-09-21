---
simd: '0065'
title: Introduce `transactionSubscribe` in PubSub Interface
authors:
  - Kirill Fomichev (kirill.fomichev@triton.one)
  - Linus Kendall (lk@triton.one)
category: Standard
type: Interface
status: Draft
created: 2023-09-21
feature: (fill in with feature tracking issues once accepted)
---

## Summary

<!-- A brief summary of what the feature is. -->

Propose a new subscription method in PubSub Interface: `transactionSubscribe`.
Clients will receive effective way to subscribe on specific transactions and
implement online data processing based on incoming selected transactions.

## Motivation

<!-- Why are we doing this? What use cases does it support? What is the expected
outcome? -->

Applications want a way to watch transactions as they happen on specific programs. Currently, 
the Solana websocket API allows to watch for changes to the program accounts and/or subscribing to logs. However,
many applicaitons depend upon the ability to receive the transactions happening for a specific program
or set of accounts. Currently, they use things like logSubscribe or polling via getTransaction, both of which are fairly
inefficient ways to achieve this purpose.

In [Solana Geyser gRPC plugin](https://github.com/rpcpool/yellowstone-grpc) by
[Triton](https://triton.one/) we implemented a filter for transactions. This filter
allow to subscribe only for specific transactions and receive them on different
commitment level. This is very useful feature which can be used by different
applications requiring online transactions processing.

This feature is already available in gRPC service but gRPC can't be used from browsers.
Triton implemented `transactionsSubscribe` method in `gRPC -> PubSub Interfance`
proxy: `Whirligig` ([docs](https://docs.triton.one/project-yellowstone/whirligig-websockets#transactionsubscribe)).

We would like to make it kind of *standard* so developers will not depend from
one data provider and can switch seamlessly to a different endpoint anytime.

## Alternatives Considered

<!-- What alternative designs were considered and what pros/cons does this feature
have relative to them? -->

Use `Solana Geyser gRPC plugin` with `Whirligig` proxy. This will require to use
`solana-validator` with `Geyser plugin` and run additional proxy software.

## New Terminology

<!-- Is there any new terminology introduced with this proposal? -->

## Detailed Design

<!--
Explain the feature as if it was already implemented and you're explaining it
to another Solana core contributor. The generally means:

- Explain the proposed change and how it works
- Where the feature fits in to the runtime, core, or relevant sub-system
- How this feature was/could be implemented
- Interaction with other features
- Edge cases
-->

Approximate code with description of parameters of new subscription method.

```rust
#[derive(Debug, Serialize)]
struct RcpTransactionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction: Option<EncodedTransactionWithStatusMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ReqTransactionFilterAccounts {
    include: Vec<String>,
    exclude: Vec<String>,
    required: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ReqTransactionFilter {
    vote: bool,
    failed: bool,
    signature: Option<String>,
    accounts: ReqTransactionFilterAccounts,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReqTransactionConfig {
    #[serde(flatten)]
    pub commitment: Option<CommitmentConfig>,
    pub encoding: Option<UiTransactionEncoding>,
    pub transaction_details: Option<TransactionDetails>,
    pub show_rewards: Option<bool>,
    pub max_supported_transaction_version: Option<u8>,
}

#[allow(clippy::needless_return)]
#[rpc]
pub trait RpcSolPubSub {
    type Metadata;

    #[pubsub(
        subscription = "transactionNotification",
        subscribe,
        name = "transactionSubscribe"
    )]
    fn transaction_subscribe(
        &self,
        meta: Self::Metadata,
        subscriber: Subscriber<RpcResponse<RcpTransactionResponse>>,
        filter: ReqTransactionFilter,
        config: Option<ReqTransactionConfig>,
    );

    #[pubsub(
        subscription = "transactionNotification",
        unsubscribe,
        name = "transactionUnsubscribe"
    )]
    fn transaction_unsubscribe(
        &self,
        meta: Option<Self::Metadata>,
        id: PubSubSubscriptionId,
    ) -> Result<bool>;
}
```

## Impact

<!-- How will the implemented proposal impacts dapp developers, validators, and core contributors? -->
Currently the pattern for many is to use signatureSubscribe + getTransaction to achieve this data. Providing transactionSubscribe allows people to refactor this pattern and just use a single pubsub method. 

## Security Considerations

<!-- What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls? -->

## Drawbacks *(Optional)*

<!-- Why should we not do this? -->

This does add a subscription method that sends a lot more data than existing signatureSubscribe.

## Backwards Compatibility *(Optional)*

<!-- Does the feature introduce any breaking changes? All incompatibilities and
consequences should be listed. -->

No changes to existing interfaces, just adding a new one.
