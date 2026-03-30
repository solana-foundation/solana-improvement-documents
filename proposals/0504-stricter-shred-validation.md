---
simd: '0504'
title: Stricter shred validation
authors:
  - Philip Taffet (Firedancer)
category: Standard
type: Networking
status: Review
created: 2026-03-13
feature: 
supersedes:
superseded-by:
extends:
---

## Summary

Shred validation currently allows certain shreds that seem like they
should be invalid.  This leads to lots of corner cases, where a new
client base their behavior on what Agave implicitly allows rather than
easily specified behavior.

## Motivation

In the course of Firedancer development, we discovered several cases of
questionable-seeming shreds that Agave accepts.  Both development teams
have agreed that the best course of action is to simplify the protocol
by rejecting these shreds.  None of these shreds would ever be produced
by a sane leader.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0317]: Enforce 32 data + 32 coding shreds**
  This further strengthens the restrictions in SIMD-0317. 

- **[SIMD-0337]: Markers for Alpenglow Fast Leader Handover**
  This proposal depends on the changes to allowable placement of
  `DATA_COMPLETE_SHRED` as dictated by SIMD-0337.  It does not depend on
  the rest of the SIMD.

[SIMD-0317]: https://github.com/solana-foundation/solana-improvement-documents/pull/0317
[SIMD-0337]: https://github.com/solana-foundation/solana-improvement-documents/pull/0337

## New Terminology

This SIMD defines the constant `MAX_SHREDS_PER_SLOT` which is currently
set to `32,768 = 2^15`.  This constant itself is not new, but this SIMD
gives it a protocol-level name.


## Detailed Design

This SIMD introduces two families of constraints for shreds.  The first
family is specific to a single shred.  Any shred failing to satisfy the
constraints in the first family MUST be ignored or discarded, and MUST
NOT be used for replay, equivocation proofs, or any other protocol
purpose.  The second family is specific to a whole FEC set.  A validator
MUST NOT vote for a block containing a FEC set failing to satisfy the
second family of constraints.


### Family 1: Shred-level checks

The checks performed on a shred are revised to the following:

1. The shred is validly signed
2. `shred_index < MAX_SHREDS_PER_SLOT`
3. `fec_set_index <= shred_index < fec_set_index + 32`
4. `fec_set_index` is a multiple of 32
5. `fec_set_index <= MAX_SHREDS_PER_SLOT - 32`
6. `variant` is chained or chained-resigned
7. Merkle height MUST be equal to 6

For data shreds:

8. `data_flags & 0xC0 != 0x80` (block complete implies batch complete)
9. If `data_flags` has batch complete, then `shred_index % 32 == 31`
10. `parent_offset <= slot`
11. For slot 0 only, `parent_offset` MUST be equal to 0.  Otherwise,
   `parent_offset` MUST NOT be equal to 0.
12. For chained data shreds, `88 <= size <= 1051`.  For chained-resigned
  data shreds, `88 <= size <= 987`.

For coding shreds:

13. `position < 32`
14. `position <= shred_index`
15. `num_data_shreds == 32` and `num_coding_shreds == 32`
16. `position == shred_index - fec_set_index`


Any shred that does not satisfy all of these constraints MUST be
immediately ignored or discarded.


For reference, the existing checks are the following, with differences
from the new checks noted at the end.

1. The shred is validly signed
2. `shred_index < MAX_SHREDS_PER_SLOT`
3. `fec_set_index <= shred_index < fec_set_index + 32`
4. `fec_set_index` is a multiple of 32
5. `fec_set_index <= MAX_SHREDS_PER_SLOT - 32`
6. `variant` is chained or chained-resigned

For data shreds:

8. `data_flags & 0xC0 != 0x80` (block complete implies batch complete)
9. If `data_flags` has batch complete, then `shred_index % 32 == 31`
10. `parent_offset <= slot`
11. For slot 0 only, `parent_offset` MUST be equal to 0.  Otherwise,
   `parent_offset` MUST NOT be equal to 0.
12. For chained data shreds, `88 <= size <= 1171 - 20*Merkle height`.
    For chained-resigned data shreds, `88 <= size <= 1107 - 20*Merkle
    height`.

For coding shreds:

13. `position < 32`
14. `position <= shred_index`
15. `num_data_shreds == 32` and `num_coding_shreds == 32`
16. `shred_index - position <= MAX_SHREDS_PER_SLOT - 32`

Note: A proof that the existing checks, excluding the signature check,
are equivalent (necessary and sufficient) to those enforced by the
combination of Agave's `must_discard_shred` and shred sanitization is
available [here](https://github.com/firedancer-io/agave/blob/ptaffet/shred-proofs/ledger/src/shred_proof.rs).

Check 7 is new.  Check 12 is just updated with the fixed Merkle height.
Check 16 is new, but combined with check 5 implies the old check 16.
Check 5 is implied by a combination of checks 2, 3, and 4, but is
included explicitly for clarity.

### Family 2: FEC-level checks

Below, a FEC set is identified by the Merkle root.  In all standard
cases, that is equivalent to identifying a FEC set by a common
signature, but a malicious leader with a specifically crafted key may be
able to produce multiple valid signatures for the same Merkle root after
[SIMD-0376](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0376-verify-strict.md)
is activated.

1. All shreds in a FEC set MUST have the same signature
2. All shreds in a FEC set MUST have the same parent Merkle root
3. All shreds in a FEC set MUST have the same {chained/unchained} and {retransmitter
   signed/retransmitter unsigned} status
4. All shreds in a FEC set MUST have the same `slot`
5. All shreds in a FEC set MUST have the same shred version
6. All shreds in a FEC set MUST have the same `fec_set_index`
7. All data shreds in a FEC set MUST have the same `parent_offset`
8. All shreds in a FEC set MUST satisfy all the constraints in Family 1

A validator MUST NOT vote for a block containing any FEC set unless it
satisfies all of these checks.  A validator SHOULD NOT replay the
transacactions in a FEC set unless it satisfies all of these checks.
Notwithstanding the Family 1 restrictions, a validator MAY use these
shreds for constructing equivocation proofs, and they MAY be served over
repair or retransmitted via Turbine.

### A note on recovery

The checks in Family 2 apply to both received and recovered shreds.
This implies that even if a validator receives all needed data shreds it
MUST ensure the coding shreds are also valid according to this SIMD, and
it MUST NOT vote on the block if the FEC set fails the check in family
2.
This extra work is necessary to eliminate the possibility of divergence
based on whether a validator received or recovered a shred.


## Alternatives Considered

The main alternative is to specify carefully exactly what shreds a
conforming validator should accept and reject.  This seems more prone to
mistakes.

## Impact

The proposal will not impact normal leaders.  Blocks produced by
malicious or extremely buggy validators may become invalid.  This will
simplify the protocol for core contributors. 

## Security Considerations

Overall, this SIMD should make security easier by removing corner cases
from the specification.  The fact that behavior is changing means that
all clients will need to implement it exactly.

## Drawbacks *(Optional)*

There's a standard philosophy in computing that software should be
maximally conservative in what it produces and maximally liberal in what
it accepts.  This SIMD goes against that, but I think that maxim does
not apply when multiple implementations need to match exactly in what
they accept.

Extra validation could have a minor performance impact.

## Backwards Compatibility *(Optional)*

Certain shreds (and thus blocks) that were previously accepted will now
be rejected.  This requires a feature gate.
