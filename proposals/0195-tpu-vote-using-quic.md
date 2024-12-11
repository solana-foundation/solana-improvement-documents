---
simd: '0195'
title: TPU Vote using QUIC
authors:
  - Lijun Wang <lijun@anza.xyz>
category: Standard
type: Core
status: Review
created: 2024-11-13
development: 
  - Anza - WIP
  - Firedancer - Not started
---

## Summary

Use QUIC for transporting TPU votes among Solana validators. This requires
supporing receiving QUIC based vote TPU packets on the server side and sending
QUIC-based TPU vote packets on the client side.


## Motivation

As timely vote credits are awarded to validators, they might be incentived to
increase the TPU vote traffic to ensure their votes are received in a timely
manner. This could cause congestions and impact overall TPU vote processing
effectiveness. The concurrent UDP based TPU vote does not have any flow control
mechanism.

We propose to apply the pattern taken for TPU transaction processing to TPU vote
processing -- by utlizing the flow control mechanism which were developed
including built-in QUIC protocol level flow control, and application-level rate
limiting on connections and packets.

## Alternatives Considered

There is no readily-available alternative to QUIC which addresses some of the
requirements such as security (reliability when applying QOS), low latency and
flow control. We could solve the security and flow control with TLS over TCP
the concern is with the latency and head-of-line problems. We could also
customize and build our own rate limiting mechanism based on the UDP directly,
this is non-trivial and cannot solve the security problem without also rely on
some sort of crypto handshaking. 

## New Terminology

In this document we define the following,
Server -- the validator receiving the TPU votes
Client -- the validator sending the TPU votes.

## Detailed Design

On the server side, the validator will bind to a new QUIC endpoint. Its
corresponding port will be published to the network in the ContactInfo via
Gossip. The client side will use the TPU vote QUIC port published by the server
to connect to the server.

The TPU vote can use the same QUIC implementation used by regular transaction
transportation. The client and server both uses their validator's
identity key to sign the certificate which is used to validate the validator's
identity especially on the server side for the purpose of provding QOS based on
the client's stakes by checking the client's Pubkey -- stake weighted QOS.

Once a QUIC connection is established, the client can send vote transaction
using QUIC UNI streams. In this design, a stream is used to send one single Vote
transaction. After that the stream is closed.

The server only supports connections from the nodes which has stakes who can
vote. Connections from unstaked nodes are rejected with `disallowed` code.

The following QOS mechanisms can be employed by the server:

* Connection Rate Limiting from all clients
* Connection Rate Limiting from a particular IpAddress
* Total concurrent connections from all clients
* Max concurrent connections from a client Pubkey.
* Max concurrent streams per connection -- this is allocated based on the ratio
of the validator's stake over the total stakes of the network.
* Maximum of vote transactions per unit time which is also stake weighted.

When the server processes a stream and its chunk, it may timeout and close the
stream if it does not receive the data in configurable timeout window.

The validator also uses gossip to pull votes from other validator. This proposed
change does not change the transport for that which will remain to be UDP based.
As the gossip based votes are pulled by the validator, the concern with
increased votes traffic is lessened.

## Impact

 QUIC compared with UDP is connection based. There is an extra overhead to
 establish the connections when sending a vote. To minimize this, the client
 side can employ connection caching and pre-cache warmer mechanism based on the
 leader schedule. Similarly the server side should maintain a sufficiently
 large enough conneciton cache for actively used connections to reduce
 connection churning and overall overhead.

## Security Considerations

The are no net new security vulnerability as QUIC TPU transaction has already
been in-place. Similar DoS attack can be targeted against the new QUIC port used
by TPU vote. The connection rate limiting can be used to fend off such attacks.

## Backwards Compatibility

Care need to taken to ensure a smooth transition into using QUIC for TPU votes
from UDP.

Phase 1. The server side will support both UDP and QUIC for TPU votes. No
clients send TPU votes via QUIC. 

Phase 2. After all staked nodes are upgraded with support of receiving TPU votes
via QUIC, restart the validators with configuration to send TPU votes via QUIC. 

Phase 3. Turn off UDP based TPU votes listener on the server side once all
staked nodes complete phase 2.