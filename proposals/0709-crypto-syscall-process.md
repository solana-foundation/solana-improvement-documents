---
simd: '0709'
title: Process for Adding Cryptographic Syscalls
authors:
  - Sam Kim (Anza)
  - ZZ (Anza)
  - Emanuele Cesena (Jump)
  - David Rubin (Jump)
category: Meta
type: Meta
status: Draft
created: 2026-10-27
---

## Summary

This proposal defines how a SIMD that adds a cryptographic syscall moves from
proposal to activation. The SIMD is merged as a Draft once it specifies the
syscall interface. Validator clients then implement the syscall behind a feature
gate while the remaining details, such as compute unit costs, are measured and
agreed on. The SIMD is then amended with those details and becomes Accepted.
Once every client implements the final details, the SIMD is Implemented and the
feature can be activated.

The testing requirements for these syscalls are defined once in this proposal so
that individual SIMDs do not need to restate them.

## Motivation

It is difficult to fully specify a cryptographic syscall before it is
implemented. The interface (the syscall signature, the input and output
encodings, and the validation rules) can be written down up front, but compute
unit costs usually cannot. They have to be benchmarked against real
implementations, and limits such as the maximum input length often depend on
those measurements. Reviewers ask for these details before accepting the SIMD,
and clients wait for acceptance before implementing, so these SIMDs stay in
review for months.

Recent SIMDs have already worked around this. [SIMD-0529] and [SIMD-0565] were
merged in June and July 2026 before their compute unit costs were final.
SIMD-0529 had preliminary costs and SIMD-0565 had none. Both were amended in
October with final costs ([#699] and [#695]). This proposal makes that flow the
standard one and states what must be true before such a SIMD is accepted and
activated. The goals are:

- The community sees the interface early and can comment on it.
- Clients can start implementing without waiting for the whole SIMD to be
  accepted.
- Testing requirements that apply to every cryptographic syscall are reviewed
  once, here, instead of in every SIMD.
- The accepted SIMD is a complete specification of the syscall, like any other
  SIMD.

## New Terminology

- Interface: the parts of a syscall's specification that determine its return
  value and output for every input. This includes the signature, the byte
  encodings, and the validation rules. It does not include compute unit costs.
- Open parameter: a part of the specification that the SIMD leaves to be filled
  in after implementation, such as a compute unit cost.
- Final amendment: the pull request that specifies every open parameter and
  moves the SIMD to Accepted.

## Detailed Design

### Scope

This process applies to SIMDs that add a syscall whose main purpose is a
cryptographic operation, such as elliptic curve arithmetic, pairings, hashing,
or signature verification. It also applies to SIMDs that extend an existing
cryptographic syscall with a new algorithm, for example a new curve ID for the
`sol_curve_*` syscalls.

A SIMD opts in by stating in its Summary that it follows this process. All other
SIMDs follow the regular process in [SIMD-0001]. This includes changes to the
compute unit costs of syscalls that are already activated.

### Lifecycle

| Step | Pull requests | SIMD status |
| --- | --- | --- |
| 1. Interface | SIMD PR with the interface | `Draft` |
| 2. Implementation | Feature-gated client PRs | `Draft` |
| 3. Testing and benchmarking | Client and conformance test PRs | `Draft` |
| 4. Final amendment | SIMD PR with the open parameters | `Accepted` |
| 5. Final implementation | Client PRs with final parameters | `Implemented` |

After step 5, the feature can be activated as described in
[Activation](#activation).

### Step 1: Interface

The author opens a PR with the SIMD and `status: Draft`. The SIMD must specify
enough of the syscall that two independent implementations return the same
value and write the same output for every input. This includes:

- The syscall signature, or the new constants (such as a curve ID) when an
  existing syscall is extended.
- The byte encoding of every input and output, including sizes, endianness, and
  the encoding of special values such as the identity point.
- Which inputs are rejected, what the syscall returns for them, and whether the
  output buffer is written on failure.
- The edge cases that implementations need to handle.

The SIMD should also include the usual Motivation, Alternatives Considered,
Impact, and Security Considerations sections. The interface PR is where the
community decides whether the network should have the syscall at all.

The SIMD may leave open parameters. These are typically:

- Compute unit costs.
- Limits that depend on measured performance, such as the maximum number of
  points accepted by a multiscalar multiplication.
- The feature gate key.

The SIMD must list each open parameter as open so that readers can tell which
parts of it are final.

Review of the interface PR covers the interface and whether to add the syscall.
Comments about open parameters are deferred to the final amendment, and testing
is covered by step 3 of this process. The PR is merged with `status: Draft` once
core contributors from each validator client with its own syscall
implementation approve it.

After the interface PR is merged, the interface should not change. If
implementation or testing shows that it must, the change must be made in an
amendment PR that is approved the same way, and every implementation must be
updated before the final amendment.

### Step 2: Implementation

Once the interface PR is merged, clients may implement the syscall and merge it
behind a feature gate. The syscall must not be callable by programs until the
feature is activated. Implementations may use provisional compute unit costs
until step 5.

The feature gate key and the tracking issue for each client should be added to
the SIMD's `feature` field when they are created, instead of waiting for
acceptance.

### Step 3: Testing and Benchmarking

Before the final amendment is merged, every implementation must pass:

- Unit tests for each edge case listed in the SIMD.
- Published test vectors for the algorithm where they exist, such as those in
  the standard that defines it or in Project Wycheproof.
- Differential fuzzing against the other client implementations that compares
  return values and output bytes for the same inputs. The fuzzing inputs must
  include malformed and boundary values. They must also exercise the syscall's
  memory handling, including invalid pointers, lengths that overflow, and
  overlapping or aliased buffers.

Each client team must confirm on the final amendment PR that its implementation
meets these requirements.

Client teams also benchmark their implementations and agree on values for the
open parameters. These discussions do not need to happen on the SIMD
repository. Their results are reviewed publicly in the final amendment.

A SIMD that follows this process does not need to describe the testing in steps
3 and 5. Its Conformance section may refer to this proposal and list only
requirements that are specific to the syscall, such as a particular source of
test vectors.

### Step 4: Final Amendment

Once the requirements of step 3 are met and the client teams agree on the open
parameters, anyone working on the syscall can open the final amendment. It
must:

- Specify every open parameter.
- Describe how the compute unit costs were derived, including the hardware,
  cluster configuration, and benchmarks used, in enough detail for others to
  check them.
- Set the `feature` field to the feature gate key and tracking issues.
- Set `status: Accepted`.

While the final amendment is open, the SIMD is in the Review stage of SIMD-0001.
The amendment is merged once core contributors from the same clients as in step
1 approve it and the requirements of step 3 are met. The SIMD is Accepted when
the amendment is merged.

Once Accepted, the SIMD must be a complete specification of the syscall. A team
writing a new validator client should be able to implement the syscall from the
SIMD alone, without reading another client's code.

### Step 5: Final Implementation

After the final amendment is merged, each client updates its implementation to
match the SIMD, including the compute unit costs and any limits set in the
amendment.

The differential fuzzing from step 3 must then be run again against the updated
implementations, and it must also compare compute units consumed. Each client
team must confirm this on the PR that sets `status: Implemented`. That PR is
merged once core contributors from the same clients as in step 1 approve it.

### Activation

The feature gate must not be activated on any public cluster until the SIMD is
Implemented. After activation on mainnet-beta, a follow-up PR moves the SIMD to
Activated as described in SIMD-0001.

### Stagnation and Withdrawal

A merged Draft under this process is not marked Stagnant while client
implementation or testing is in progress.

If the client teams decide not to ship the syscall, the SIMD moves to Withdrawn
and its feature gate is never activated.

## Alternatives Considered

Keep the current process, where the whole SIMD, including compute unit costs, is
accepted before implementation starts. This keeps a single review, but the
costs cannot be set without an implementation, so in practice these SIMDs wait
in review for months.

Implement first and write the SIMD afterwards. This is the fastest path, but the
community only sees the syscall after its interface is fixed in client code, and
reviewers can only approve what already exists.

Merge the interface SIMD as Accepted with open parameters. This also lets
clients implement early, but Accepted would then mean different things for
different SIMDs, and the status would no longer tell readers whether the
specification is final.

Add a new status to SIMD-0001, such as "Interface Accepted". This is more
explicit, but it requires changing SIMD-0001 and the repository linter, and
Draft is already used for merged SIMDs that are not final.

## Impact

Client teams can start implementing a cryptographic syscall as soon as its
interface is reviewed instead of after the whole SIMD is accepted. Reviewers of
the interface PR do not need to evaluate compute unit costs or testing plans.

Application developers can see the interface early and prototype against it.
Compute unit costs are not final until the final amendment, so programs should
not rely on provisional costs.

## Security Considerations

Under this process, syscall code lands in validator releases before its
specification is final. The feature gate is what keeps that code off the
network, so the syscall must not be reachable by programs before activation, and
the feature must not be activated before the SIMD is Implemented.

Compute unit costs that are too low let programs use more validator time than
they pay for, which can delay replay and block production. This process moves
the review of the costs to the final amendment but does not skip it. The
amendment has to show how the costs were measured, and it is reviewed publicly
before activation.

Clients that charge different compute units for the same call can disagree on
whether a transaction exceeds its compute budget. Step 5 checks that every
client charges the approved costs before the feature is activated.

The testing requirements in steps 3 and 5 are a minimum. A SIMD may add
requirements for its syscall and should do so when the algorithm has known
implementation pitfalls.

[SIMD-0001]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0001-simd-process.md
[SIMD-0529]: https://github.com/solana-foundation/solana-improvement-documents/pull/529
[SIMD-0565]: https://github.com/solana-foundation/solana-improvement-documents/pull/565
[#695]: https://github.com/solana-foundation/solana-improvement-documents/pull/695
[#699]: https://github.com/solana-foundation/solana-improvement-documents/pull/699
