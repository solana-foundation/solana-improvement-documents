---
simd: '0195'
title: TPU Feedback
authors:
  - Lijun Wang <lijun@anza.xyz>
category: Standard
type: Core
status: Review
created: 2025-02-14
development: 
  - Anza - WIP
  - Firedancer - Not started
---

## Summary

This document defines the protocol to send feedback on TPU processing from the
servers to the clients.

## Motivation

A client sends TPU transactions to the leader as QUIC uni-streams. Currently we
only send transport level acknowledememt to the clients. I.e we only tell if a
transaction has been received by the server. A transaction received by the
server may not be processed due to resource contention. For example, if there
are too many transactions with higher paying fees than this transaction, the
transaction is not included in the block. Providing further information on the
transaction state will help the client to make more informed decision on
handling transaction failures and reduce the incentive to blindly resubmitting
transactions and help reduce congestion condition in the network.

## Alternatives Considered

There are different transport mechanism to send the feedback such as
bidrectional streams, reverse unistream from the server to the client and
QUIC datagrams. We choose datagrams due to its simplicity. One reason we have
used stream over datagrams is the expectation that transaction size can be
increased in the future which can not be carried in one datagram. Sending
feedback has no such concern as we can send multiple datagrams if there are
too many transactions to acknowledge.

## New Terminology

In this document we define the following,

TPU Feedback -- information sent by the server about transaction processing.
Server -- the validator receiving the TPU transactions and sending the feedback.
Client -- the validator or application using QUIC TPU client sending the TPU
          transactions and receiving the feedback.

## Detailed Design

When the client sends transactions to the server, it must first open a QUIC
connection to the server. The feedback is sent by the server using the same
connection. The feedback is sent using a QUIC datagram. The client wishing to
receive feedback from the server must be ready to receive application datagrams
from its connections.

A client connection notifies the server of its interest to receive feedback by
sending a datagram soliciting feedback. This helps server to avoid sending
datagrams to clients which are not interested in receiving feedback or clients
which are not upgraded to have such capabilities.

The client and server must enable the support of datagrams QUIC frame types
via transport parameter `max_datagram_frame_size`. 

The server sends the feedback datagrams to interested clients periodically.
The frequency is decided by the server and consideration should be given to
coalesce feedback on more transactions to reduce datagram counts and short
enough feedback latency. For example 100ms can be considered. If there are
more transactions to be acknowldged than the one in one datagram, more datagrams
can be sent without waiting for the feedback interval.

### The format of the TPU feedback

struct TpuFeedback {
  version: u32,
  timestamp: u32,
  transaction_state: TransactionState,
  priority_fee_info: PriorityFeeInfo
}

The version is a 32 bit unsinged integer, it is set to 1 for first version.
The timestamp is the 32 bit unsigned integer representing the number of seconds
since midnight 01/01/1970.

TransactionState is defined as the following:

struct TransactionState {
  transactions_count: u8
  (transaction, state) pairs
}

The transactions_count is 8 bit unsgined integer representing the count of
transactions in this feedback. It is followed by the count of
(transaction, state) pairs. Each transaction is represented by the first 8 bytes
of the transaction signature and each state is represented by an 32 bit unsigned
integer.

The transaction state value is defined as the below enum:

enum TransactionStateValue {
  Packed = 0,
  FeeTooLow = 1,
  FeePayerBalanceTooLow = 2,
}

Newer state value can be defined as needed in the future with its own unique
value. Client should disregard the value if it does not understand its meaning.

The proirity fee information is defined as the following,

struct PriorityFeeInfo {
  entries_count: u8
  priority_fee_stats: 
}

## Impact

## Security Considerations

## Backwards Compatibility


## References

An Unreliable Datagram Extension to QUIC:
https://datatracker.ietf.org/doc/html/rfc9221
