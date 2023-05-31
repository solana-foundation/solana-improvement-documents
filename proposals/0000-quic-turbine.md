---
simd: '0053'
title: Block Propagation Using QUIC Protocol
authors:
  - Solana Labs
category: Standard
type: Networking
status: Draft
created: 2023-05-31
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Use QUIC protocol (instead of current UDP implementation) to propagate blocks
across cluster.

## Motivation

* Compared to UDP, QUIC provides better features to preserve QoS.
* Verifying Turbine propagation path (see
  [solana/#28384](https://github.com/solana-labs/solana/issues/28384) and
  [solana/#20969](https://github.com/solana-labs/solana/issues/20969)) requires
  some mechanism to prevent IP address spoofing, which is readily available
  with QUIC.

## Alternatives Considered

An alternative would be to keep current UDP implementation but,

* Implement [ping/pong
  protocol](https://github.com/solana-labs/solana/blob/2fc1dc1bf/gossip/src/ping_pong.rs)
  between nodes to verify IP addresses before retransmitting any shreds to the
  recipient.
* Each node also signs and attaches its signature to each shred it is
  retransmitting.

This will cost 64 bytes of each shred payload to embed retransmitter's
signature and an extra signature verification overhead for each shred at the
recipient node.

There are ways to improve above but it gradually approaches to re-inventing
QUIC which defeats the point of not using QUIC to begin with.

## New Terminology

TVU QUIC socket address refers to the port which validator nodes will receive
shreds using QUIC protocol.

## Detailed Design

Once the cluster has upgraded to the new
[`ContactInfo`](https://github.com/solana-labs/solana/blob/2fc1dc1bf/gossip/src/contact_info.rs#L68-L85)
nodes may explicitly specify a socket address for TVU QUIC connections.
Until then, similar to QUIC migration for TPU, we will use the port at
[`QUIC_PORT_OFFSET`](https://github.com/solana-labs/solana/blob/2fc1dc1bf/sdk/src/quic.rs#L4)
from the TVU socket for QUIC connections.

For the initial implementation in the Solana Labs client we will use the same
constructs as TPU QUIC implementation so the specs are the same. See [TPU/QUIC
Protocol v1](https://github.com/solana-foundation/specs/blob/42f2058b7/p2p/tpu.md#tpuquic-protocol-v1).

In terms of Solana Labs client design specifically, this would entail:

* Spinning up a new [QUIC
  server](https://github.com/solana-labs/solana/blob/2fc1dc1bf/streamer/src/quic.rs#L393-L406)
  in TVU stage to ingest shreds and channel through to
  [shred-fetch-stage](https://github.com/solana-labs/solana/blob/2fc1dc1bf/core/src/shred_fetch_stage.rs).
* Retransmit shreds through a
  [QUIC-connection-cache](https://github.com/solana-labs/solana/blob/master/quic-client/src/lib.rs)
  in [retransmit-stage](https://github.com/solana-labs/solana/blob/master/core/src/retransmit_stage.rs).


## Impact

Validators should allocate TVU +
[`QUIC_PORT_OFFSET`](https://github.com/solana-labs/solana/blob/2fc1dc1bf/sdk/src/quic.rs#L4)
port for TVU QUIC connections.

## Security Considerations

We expect migrating from UDP to QUIC will provide useful features in improving
security and QoS of the block propagation protocol.

## Drawbacks

Obviously QUIC will add extra overheads compared to a bare UDP implementation.
We expects these overheads will be mitigated by using a large enough
connection-cache to reuse the same connections and minimize handshake costs.


## Backwards Compatibility

The change is inherently backward incompatible and requires a new P2P protocol
implementation between nodes.
