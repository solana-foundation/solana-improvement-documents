---
simd: '0042'
title: Fraud Proofs for SVM
authors:
  - Anatoly Yakovenko
category: Standard/Meta
type: Core
status: Draft
created: 2023-8-9
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This SIMD proposes a design for adding succinct proofs of invalid
computation to Solana's SVM. This would make it much easier to
verify the correctness of transactions, and to detect fraud or
malicious activity.


## Motivation

With fraud proofs, honest participants in the network will be able
to construct and gossip a fraud proof that can be easily verified
in a browser that prooves that the majority is faulty.  Once the
fraud proof is detected, clients can disconnect from the cluster.

## Alternatives Considered

None

## New Terminology

SPV: Simplified Payment Verification.  This industry standard acronym
describes a merkle path from a transaction to a set of quorum votes
that finalize the transaction.

## Detailed Design

### SPV Inclusion Criteria

zsh:1: command not found: fm
1. The account hash for every input account (both read and write operations).
2. The transaction count that led to the generation of that account hash.
3. The current transaction count.
4. The outcome or result of the transaction.

### Validation Mechanism

There are two primary scenarios that need to be addressed:

1. Incorrect Transaction Computation by Majority: If the majority
computes this transaction incorrectly, it becomes straightforward
to verify this discrepancy, even on platforms with minimal computational
power, like a web browser. Users can download the program, and all
the inputs, verifiy all the inputs against the account hashes, then
compute the result and see that the majority signed an invalid
outcome.

2. Use of Outdated Account Version by Majority: If the majority
uses an older version of an account, this can also be easily
validated. This is achieved by merely requiring an SPV to demonstrate
a more recent successful transaction that updated the concerned
account.

### Quorum changes and Epochs

Light clients need to track epoch rollover and update their view of the quorum.

## Impact

- Replay stage needs to assign a deterministic transaction count to each transaction.
- Accounts db needs to store the transaction count that last modified the account.
- Sysvars and system calls that read values must be tracked within the SPV as well.

## Security Considerations

This improves detection of invalid state transitions on solana.

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

NA
