---
simd: 'XXXX'
title: Title of SIMD. This should be concise and descriptive.
authors:
  - (fill in with names of authors)
category: Standard/Meta
type: Core/Networking/Interface/Meta
created: (fill me in with today's date, YYYY-MM-DD)
feature: (fill in with feature key and github tracking issues once accepted)
supersedes: (optional - fill this in if the SIMD supersedes a previous SIMD)
superseded-by: (optional - fill this in if the SIMD is superseded by a subsequent
 SIMD)
extends: (optional - fill this in if the SIMD extends the design of a previous
 SIMD)
---

## Summary

A brief summary of what the feature is.

## Motivation

What problem does this proposal solve? Describe the problem on its own terms,
independently of the solution proposed by this SIMD. Who is affected, in what
way, and why the status quo is not sufficient? A reader should be able to use
this section to explore the solution space on their own, not just evaluate the
solution proposed here in isolation.

What is the expected outcome after this proposal is implemented?

## Dependencies *(Optional)*

This proposal depends on the following previously accepted proposals:

- **[SIMD-XXXX]: Title of SIMD**

    A brief summary of the reason for the dependency

[SIMD-XXXX]: https://github.com/solana-foundation/solana-improvement-documents/pull/XXXX

## New Terminology

Is there any new terminology introduced with this proposal?

## Detailed Design

Explain the feature as if it was already implemented and you're explaining it
to another Solana core contributor. This generally means:

- Explain the proposed change and how it works
- Where the feature fits in to the runtime, core, or relevant sub-system
- How this feature was/could be implemented
- Interaction with other features

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC
2119](https://www.ietf.org/rfc/rfc2119.txt) and [RFC
8174](https://www.ietf.org/rfc/rfc8174.txt).

### Edge Cases

Which edge cases should the implementors of this change be aware of? Enumerate
these below. This may include topics such as extreme max/min/zero values,
unusual states and scenarios that require particular attention.

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                              |
|---------------------------------|-------------------------------------|
| Transaction Execution (Runtime) |                                     |
| Virtual Machine                 |                                     |
| Block Packing                   |                                     |
| Consensus                       |                                     |
| Gossip                          |                                     |
| Turbine                         |                                     |
| Snapshots                       |                                     |
| On-Chain Core BPF Programs      |                                     |
| Other (please describe)         |                                     |

## Alternatives Considered

Describe fundamentally different approaches that could address the problem
identified in the motivation section (i.e. the solution space), and explain
why the proposed design was chosen over these alternative solutions.

## Impact

How will the implemented proposal impact dapp developers, validators, and core
contributors?

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

Does the feature introduce any breaking changes? All incompatibilities and
consequences should be listed.

## Conformance

How will all clients verify that they have correctly implemented this change?

Some examples:

- The change will be accompanied by a ledger from a
  localnet cluster demonstrating the behaviour prior to
  activation, the feature activation, and change in
  functionality caused by the activation. An example of
  how to create such a ledger:
  https://github.com/febo/p-token-migration
- The change will be accompanied by a pcap which contains
  the packets that are handled differently after the
  feature gate is activated.
