---
simd: '0674'
title: Validator Location Registration
authors:
  - Quentin Kniep
  - Roger Wattenhofer
category: Standard
type: Core
status: Draft
created: 2026-09-24
feature: (fill in with feature key and github tracking issues once accepted)
extends: SIMD-0185
---

## Summary

Validators register their physical location as an integer ECEF
coordinate triple in their vote account, via a new Vote program
instruction. ECEF (Earth-Centered, Earth-Fixed) is the established
geodesy standard for expressing a position as Cartesian coordinates in
meters from the earth's center of mass. All consensus-relevant
computation is exact 64-bit integer arithmetic: no floating point
arithmetic and no trigonometry. The registered locations and the
distance measure are the input to a geo-aware leader schedule, which is
specified in a separate SIMD.

This proposal defines two feature gates:

1. `validator_location_registration` enables the `UpdateLocation`
   instruction.
2. `enforce_validator_location_registration` makes registration
   mandatory: vote accounts without a registered location are no longer
   admitted to the active validator set.

## Motivation

A geo-aware leader schedule reorders the leader schedule so that
consecutive leaders tend to be geographically close, reducing the
average handoff latency between leaders. Any such schedule needs two
primitives that every validator computes identically: a self-reported
location per validator, and a distance measure.

Floating-point trigonometry (e.g. the haversine formula on
latitude/longitude) is a consensus hazard, since results may differ
across libraries, compilers and architectures. This proposal instead
uses the ECEF integer coordinate format. ECEF is supported by every GPS
library. Based on these ECEF coordinates, we get integer distances
whose comparisons are provably order-identical to true great-circle
distances, so all schedule computations are bit-reproducible on every
client.

## Dependencies

This proposal stores the location in Vote Account v5, defined by
[SIMD-0701] as the successor of [SIMD-0185] (Vote Account v4).
[SIMD-0701] also defines how a Vote Account v4 is converted to Vote
Account v5, and the upgrade of the Stake program when
`validator_location_registration` activates. Vote Account v5 has no
tower vote state, so this proposal requires the Alpenglow migration
([SIMD-0384]) to have succeeded.
`enforce_validator_location_registration` filters the active validator
set defined by [SIMD-0357] (Alpenglow Validator Admission Ticket). It
follows the pattern
of [SIMD-0387] (BLS Pubkey Management in Vote Account), which likewise
registers a validator key in the vote account and makes registration
mandatory before a consuming feature activates. The companion geo-aware
leader schedule [SIMD-0675] depends on this proposal.

[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/pull/185
[SIMD-0387]: https://github.com/solana-foundation/solana-improvement-documents/pull/387
[SIMD-0675]: https://github.com/solana-foundation/solana-improvement-documents/pull/675
[SIMD-0357]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0357-alpenglow_validator_admission_ticket.md
[SIMD-0384]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0384-alpenglow-migration.md
[SIMD-0701]: https://github.com/solana-foundation/solana-improvement-documents/pull/701

## New Terminology

- **Location registration**: the integer ECEF coordinate triple a
  validator stores in its vote account.
- **Validity margin**: the maximum absolute divergence from the WGS84
  ellipsoid accepted by the integer surface check (roughly 1 km around
  the WGS84 ellipsoid).
- **dist2**: the squared Euclidean chord distance between two
  locations, in square meters, as a u64.

## Detailed Design

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and
"OPTIONAL" in this document are to be interpreted as described in
[RFC 2119](https://www.ietf.org/rfc/rfc2119.txt) and
[RFC 8174](https://www.ietf.org/rfc/rfc8174.txt).

### Registration format

A location registration is three signed 32-bit integers:

```
coords = (x: i32, y: i32, z: i32)   // ECEF, in meters
```

Coordinates are WGS84 ECEF in meters, rounded to the nearest integer,
at ellipsoid height zero: the registered point is the point on the
ellipsoid surface beneath the validator's machine, ignoring its
altitude. WGS84 is defined by [NGA.STND.0036_1.0.0_WGS84][wgs84]
(Department of Defense World Geodetic System 1984); the geocentric ECEF
frame is EPSG:4978. All ellipsoid constants in this document follow
that standard.

The ellipsoid is roughly sea level, not the ground: a validator on high
ground MUST still use height 0, otherwise its location may fall outside
the validity margin.

Each of the three components x, y, z, individually, MUST have an
absolute value of at most `6_378_138`.
`6_378_138` is the WGS84 semi-major axis plus 1 m: no point on the
ellipsoid has a coordinate of larger magnitude (`|x|, |y| <= a`,
`|z| <= b < a`).

[wgs84]: https://earth-info.nga.mil/php/download.php?file=coord-wgs84

### Deriving the coordinates

Validators MAY derive their registration off-chain from geodetic
latitude `phi` and longitude `lambda` (WGS84, with height 0); floating
point is acceptable here because only the rounded ECEF integers are
registered:

```
a  = 6_378_137.0                     // WGS84 semi-major axis, m
e2 = 6.694_379_990_14e-3             // first eccentricity squared
N  = a / sqrt(1 - e2 * sin(phi)^2)

x = round( N * cos(phi) * cos(lambda) )
y = round( N * cos(phi) * sin(lambda) )
z = round( N * (1 - e2) * sin(phi) )
```

This is the standard geodetic-to-ECEF conversion (with `h = 0`)
available in every GPS/geodesy library. Registrations can be checked by
hand against public converters. Enter latitude, longitude and height 0,
then round the resulting X, Y, Z (meters) to integers:

- [NOAA NGS XYZ conversion](https://geodesy.noaa.gov/TOOLS/XYZ/xyz.shtml)
- [GNSS Coordinate Converter](https://gnsscalc.com/coordinates/)
- [NPS LLH to XYZ](https://www.oc.nps.edu/oc2902w/coord/llhxyz.htm)

### Validity check

Every client MUST reject a registration unless it passes the following
check, computed entirely in u64/i64 integer arithmetic.

```
// constants
A2  = 40_680_631_590_769   // a^2
K   = 442                  // round( (a^2 - b^2) / b^2 * 2^16 )
TOL = 12_756_274_000       // ~ 2 * a * 1000: a 1 km tolerance

// check (all in u64; components widened from i32)
q = x*x + y*y + z*z + ((K * (z*z)) >> 16)
valid  <=>  |q - A2| <= TOL   and   |x|,|y|,|z| <= 6_378_138
```

`K / 2^16` is a fixed-point encoding of the ellipsoid constant
`(a^2 - b^2) / b^2 ~ 0.006739`. The shift is an exact division by
`2^16`, and the tolerance absorbs rounding and the quantization of `K`.
Accepted points lie within a margin of roughly 1 km around the
ellipsoid surface, so co-located validators are still in the same city.

The coordinate bound MUST be checked first. It keeps the numbers small
enough that computing `q` cannot overflow: within the bound,
`K * z*z < 2^55` and `q < 2^47`.

### On-chain registration mechanism

The location is stored in the validator's vote account. The Vote
program gains one instruction, a new `VoteInstruction` variant (Rust
syntax, serialized like the existing variants):

```rust
UpdateLocation { x: i32, y: i32, z: i32 }   // ECEF meters
```

- The instruction MUST be signed by the vote account's authorized
  voter (vote authority).
- The runtime MUST fail the instruction unless the coordinates pass
  the validity check above; a vote account therefore never holds an
  invalid location.
- The coordinates are stored in the `location` field of a Vote Account
  v5, defined in [SIMD-0701]. Older vote account versions have no
  location, and are treated as not having a registered location.
  Since the origin fails the validity check, (0, 0, 0) can never be a
  valid registration, and
  `UpdateLocation` can never store it.
- Registration becomes mandatory with a second feature gate, which
  MUST be activated before any consensus-relevant logic (such as the
  geo-aware leader schedule) consumes locations. From then on, a
  validator without a registered location is treated as if it were not
  staked: it is excluded from the leader schedule and from voting, and
  its stake does not count toward consensus, effective at the start of
  the epoch.
  The second gate is `enforce_validator_location_registration` (see
  [Mandatory registration](#mandatory-registration)).
- The leader schedule of epoch `e+1` is computed at the start of epoch
  `e`, using the location stored in the vote account at that moment,
  the same moment the schedule's stake weights are read.
  Consumers MUST take the location from the vote account state captured
  with the epoch stakes, not from the current account, because a
  schedule can be computed later than that moment (for example after
  booting from a snapshot). Clients therefore keep each validator's
  location with its epoch stakes, as they keep its node identity.
  `UpdateLocation` MAY be submitted at any time and any number of
  times, but only the value visible at a schedule computation counts:
  a change submitted during epoch `e-1` is first reflected in the
  leader schedule of epoch `e+1`, with the same latency as a stake
  delegation. An already computed schedule is never adapted; within an
  epoch, every validator's location is fixed.
- The new field adds 12 bytes to the vote state (three i32).

A new instruction, `UpdateLocation`, is added to `VoteInstruction` with
discriminant 20 (the next unused index; appended after all existing
variants).

```
Instruction data (16 bytes):
  0   4   discriminant (u32 LE) = 20
  4   4   x (i32 LE)
  8   4   y (i32 LE)
  12  4   z (i32 LE)

Accounts:
  0 [writable]  vote account
  1 [signer]    vote authority
```

Errors:

- vote account not owned by the vote program: `InvalidAccountOwner`
- account 1 is not the vote authority: `MissingRequiredSignature`

Trailing bytes after the 16 bytes above are ignored, as for every other
vote instruction.

The Vote program MUST process `UpdateLocation` in the following order.
Where a step fails, the instruction fails with the listed error and the
account is not modified.

1. Check that the vote account is owned by the Vote program (the
   existing check, done before decoding). Error: `InvalidAccountOwner`.
2. Decode the instruction data, and check that
   `validator_location_registration` is active. Error:
   `InvalidInstructionData`.
3. Deserialize the vote state. An uninitialized account fails. Vote
   account versions older than v4 are converted to Vote Account v4 in
   memory, as in [SIMD-0185]; Vote Account v4 and Vote Account v5 are
   used as stored. Error: `InvalidAccountData`.
4. Look up the authorized voter for the current epoch (`Clock` sysvar)
   with the existing authorized-voter lookup, which also purges expired
   entries. That voter MUST have signed the transaction. Error:
   `MissingRequiredSignature` (and the lookup's own errors, as today).
5. Check that the coordinates are a valid location. Error:
   `InvalidArgument`.
6. If the account is not a Vote Account v5, check that the account
   data is exactly 3762 bytes and convert it to Vote Account v5 as
   specified in [SIMD-0701]. The conversion fails if the account has no
   BLS key, more than 4 authorized voters or more than 64 epoch credits
   entries. Error: `InvalidAccountData`.
7. Set `location` to the coordinates.
8. Store the vote state through the existing `set_state` path, including
   its writable and ownership checks, with their errors as today.

On success the account is a Vote Account v5 with the new location, and the
purge from step 4 is saved. If it was already a Vote Account v5, only the
location changes. The usual compute budget applies.

### Mandatory registration

When `enforce_validator_location_registration` is active, a vote account
is admitted to the active validator set ([SIMD-0357]) only if, besides the
existing rules, it is a Vote Account v5 that deserializes. Deserialization
checks the location ([SIMD-0701]), so every admitted account has a valid
location.

- An excluded account is treated as unstaked: it gets no leader windows,
  its votes don't count, and its stake doesn't count towards consensus or
  the total stake.
- The rule applies from the set computed at the boundary where the
  feature activates. Earlier sets are not recomputed.
- Snapshots store the filtered sets with the vote account state they came
  from. A node booting from a snapshot uses them as stored, without
  checking admission again.

### Distance

The distance measure between two valid registrations `u` and `v` is the
squared straight-line (chord) distance:

```
dist2(u, v) = dx*dx + dy*dy + dz*dz          // u64
  where dx = i64(u.x) - i64(v.x), etc.
```

Differences MUST be computed in i64 before squaring. The maximum
possible value is the squared diameter, about 1.63 * 10^14 < 2^48, so
all intermediate sums and `dist2` fit u64.

The chord is strictly monotone in the great-circle angle between two
surface points. Therefore comparing `dist2` values yields exactly the
same order as comparing true surface distances. The geo-aware leader
schedule consumes distances only through comparisons, so the schedule
computed from `dist2` is identical to one computed from exact
great-circle distances, while requiring no square roots, no
trigonometry and no floating point.

### Test vectors

Derived with the formulas above (lat, lon in degrees; h = 0):

| location  | lat    | lon     | x        | y        | z       |
|-----------|--------|---------|----------|----------|---------|
| Frankfurt | 50.110 |   8.680 |  4051542 |   618526 | 4870645 |
| London    | 51.510 |   0.000 |  3977778 |        0 | 4969055 |
| New York  | 40.710 | -74.010 |  1333729 | -4654333 | 4138065 |
| Tokyo     | 35.680 | 139.690 | -3955214 |  3355452 | 3699409 |

The corresponding check deviations |q - A2| are 116_387_798
(Frankfurt), 122_847_190 (London), 87_098_983 (New York) and
72_402_516 (Tokyo). In this example, London is registered on the prime
meridian (Greenwich), so its y component is exactly 0.

All pass the validity check (deviations far below TOL). The origin
(0, 0, 0) fails with |q - A2| = A2. Distances:

```
dist2(Frankfurt, New York) = 35_726_222_993_250
dist2(Frankfurt, Tokyo)    = 72_970_699_340_708
```

Further validity check cases:

| location | valid | reason |
|----------|-------|--------|
| `(6_378_138, 0, 0)`, `(0, -6_378_138, 0)` | yes | on the bound |
| `(6_378_139, 0, 0)` | no | outside the bound, within the tolerance |
| `(0, -6_378_139, 0)` | no | outside the bound, within the tolerance |
| `(4_875_666, 4_113_547, 2)` | yes | `q - A2 = TOL` |
| `(4_719_095, 4_292_264, 7)` | no | `q - A2 = TOL + 1` |
| `(4_846_432, 4_144_873, 4)` | yes | `A2 - q = TOL` |
| `(4_554_754, 4_463_417, 19)` | no | `A2 - q = TOL + 1` |
| any coordinate `i32::MIN` or `i32::MAX` | no | outside the coordinate bound |

### Edge Cases

- Two validators MAY register identical coordinates; `dist2 = 0` is
  valid and requires no special handling.
- The poles and the antimeridian need no special casing; ECEF has no
  coordinate seams.

### Validator Components Affected

| Validator Component             | Impact                              |
|---------------------------------|-------------------------------------|
| Transaction Execution (Runtime) | Vote program: new ix, state field   |
| Virtual Machine                 | None                                |
| Block Packing                   | None                                |
| Consensus                       | dist2; VAT requires location        |
| Gossip                          | None                                |
| Turbine                         | None                                |
| Snapshots                       | None                                |
| On-Chain Core BPF Programs      | Stake program upgrade (SIMD-0701)   |

## Alternatives Considered

- **Latitude/longitude fixed point (degrees * 10^7) with haversine.**
  Human-readable and standard on GPS receivers, but the distance
  computation is trigonometric, and in consensus that means a bespoke
  integer implementation of sin and asin (floating point is not an
  option), which every client would have to reproduce bit-identically.
  ECEF needs no trigonometry in consensus at all.
- **Latitude/longitude with integer equirectangular distance.** Simple,
  but needs a cos(latitude) table, special-casing at the antimeridian,
  and its distortion grows with distance, so distance *orderings* can
  differ from true great-circle order at continental scale.
- **Unit-sphere fixed point (scale 2^25).** Mathematically cleanest
  (perfectly spherical validity check), but a bespoke format nobody
  else implements; ECEF meters is the recognized standard with the same
  arithmetic properties.
- **u32-only arithmetic (scale ~2^15, i16 components).** Fits all
  values in u32 and halves the registration to 6 bytes, at the cost of
  ~200 m coordinate resolution and overflow margins tight enough to
  invite implementation bugs. 64-bit arithmetic has no overhead on
  validator hardware.
- **Sphere-only validity check (\|q - R0^2\| <= tol).** Simpler
  constant, but requires a ~+/-25 km tolerance to cover the ellipsoid's
  polar flattening, letting co-located validators disagree by up to
  ~50 km. The exact ellipsoid check costs one extra multiply and
  shrinks the margin to 1 km.

## Impact

Validators register a 12-byte coordinate triple once, and again
whenever they move; clients verify registrations with a handful of
integer operations. Dapp developers are unaffected. Core contributors
gain a deterministic location and distance primitive on which the
geo-aware leader schedule ([SIMD-0675]) is built; the primitive is also
reusable for network diagnostics and telemetry.
Registering a location converts the vote account to Vote Account v5.

## Security Considerations

- **The check verifies geometry, not actual location truthfulness.** A
  validator can register any point on the surface, including one far
  from its actual machine; countering location lies is the
  responsibility of the consuming leader schedule design.
- **Authenticity and replay protection** come from the existing vote
  account machinery: `UpdateLocation` is an ordinary signed
  transaction, and only the vote authority can change the stored
  location.
- **Precision is intentionally coarse.** Meter-level coordinates
  reveal no more than existing gossip IP addresses already do; a
  validator MAY round its registration to city-level precision, the
  schedule outcome is essentially unchanged.
- **Determinism.** All validation and distance computation is exact
  integer arithmetic with specified widths; there is no
  implementation-defined behavior to diverge on.
  The evaluation order is specified as well.
- **Mandatory registration removes unregistered stake.** Activating
  `enforce_validator_location_registration` before most stake has
  registered removes that stake from consensus; if no stake has
  registered, the cluster halts.

## Drawbacks

Setting the location information requires manual operator intervention.

The feature gate for making location information mandatory needs to be
planned, to make sure validators are not accidentally excluded from
consensus.

## Backwards Compatibility

Additive and feature-gated. The vote state gains a new version with
the location field; tools that parse vote accounts must learn the new
layout, as with previous vote state version bumps. Before activation
of the consuming leader schedule feature, registered locations have no
protocol effect other than mandatory registration. No other existing
behavior changes.

The features MUST be activated in this order, with at least a 1 epoch
wait between them.

1. `alpenglow`
2. `validator_location_registration`
3. `enforce_validator_location_registration`, after operators have had
   time to register.

## Conformance

A correct implementation MUST accept the four test vectors above with
exactly the listed `q` deviations, MUST reject the origin `(0, 0, 0)`,
MUST classify the further validity check cases as listed, and MUST
reproduce the two `dist2` values exactly.

A series of test fixtures MUST be provided, using the
[conformance tooling](https://github.com/anza-xyz/agave/wiki/Conformance-Tools).
These MUST include the following scenarios and edge cases at a minimum:
`UpdateLocation` before and after `validator_location_registration`,
on vote accounts older than v4, on
Vote Account v4 (with and without a BLS key, with the wrong account
length) and on Vote Account v5, with each error in the step table, with
trailing instruction data, and checking that the authorized-voter purge
is persisted. A local cluster ledger will activate both gates and show
the active validator set before and after
`enforce_validator_location_registration`.
