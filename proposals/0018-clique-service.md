---
simd: '0018'
title: Clique Service
authors:
  - Maximilian Schneider (Mango)
category: Networking
type: Optional
status: Draft
created: 2023-01-13
---

## Problem

Various users don’t like the probabilistic nature of turbine, which can lead to two issues: 

1. Generally high latency for RPC nodes in Eastern Asia (where most popular crypto currency exchanges are hosted)
1. Spikes in `retransmit-stage-slot-stats.elapsed_millis` for RPC nodes, which causes delays on processing of following slots.

A common workaround is to run a high stakes validator with a patch that forwards shreds directly to a selection of unstaked RPCs, in addition to the regular turbine path. This method is called “shredstream forward” and is used by Mango, Triton & Jito in production. Enabling forwards from a single staked validator in L1 of turbine can reduce latency by multiple seconds. It’s an incredibly effective method to improve an unstaked RPC node’s latency with the result is that it behaves nearly as good as if physically located in Europe.

The issue with this solution is that it’s currently cumbersome to configure and hence only star shaped deployments are known to me. In addition it is very inaccessible, as it is unknown to most users and requires direct access to a high stakes validator to deploy a patched version of the solana client.

## Solution

Create an opt-in clique service, which in parallel to retransmit stage shares every shred received with a clique of validators. To allow for easy configuration of different network topologies the clique service can choose between different way to discover other clique members:

1. public coordination servers (similar to ipfs), mango would run such a server to improve distribution of shreds in south-east Asia
1. private lists of ips, service providers like triton could continue to run a private high performance version

I would prefer to use an existing gossip network implementation, that has been battle tested over hand-rolling a third p2p network (gossip, turbine, …) and use libp2p’s [gossipsub](https://github.com/libp2p/specs/tree/master/pubsub/gossipsub). In practice I expect many users to roll their own optimized implementation, but at least now they have a central point they can just override, rather than having to manually patch diferent points in the code.
