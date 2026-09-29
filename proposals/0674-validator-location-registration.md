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

This proposal extends [SIMD-0185] (Vote Account v4) by introducing the
next vote state version on top of the V4 layout. It follows the pattern
of [SIMD-0387] (BLS Pubkey Management in Vote Account), which likewise
registers a validator key in the vote account and makes registration
mandatory before a consuming feature activates. The companion geo-aware
leader schedule SIMD-0XXX depends on this proposal.

[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/pull/185
[SIMD-0387]: https://github.com/solana-foundation/solana-improvement-documents/pull/387

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

Each of the three components x, y, z, individually, MUST have an
absolute value of at most `6_378_138`.

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

### On-chain registration mechanism

The location is stored in the validator's vote account. The Vote
program gains one instruction, a new `VoteInstruction` variant (Rust
syntax, serialized like the existing variants):

```rust
UpdateLocation { x: i32, y: i32, z: i32 }   // ECEF meters
```

- The instruction MUST be signed by the vote account's validator
  identity.
- The runtime MUST fail the instruction unless the coordinates pass
  the validity check above; a vote account therefore never holds an
  invalid location.
- The coordinates are stored as a new field of the vote state
  (feature-gated new vote state version), initialized to (0, 0, 0),
  which denotes that no location is registered. Since the origin fails
  the validity check, (0, 0, 0) can never be a valid registration, and
  `UpdateLocation` can never store it.
- Registration becomes mandatory with a second feature gate, which
  MUST be activated before any consensus-relevant logic (such as the
  geo-aware leader schedule) consumes locations. From then on, a
  validator without a registered location is treated as if it were not
  staked: it is excluded from the leader schedule and from voting, and
  its stake does not count toward consensus, effective at the start of
  the epoch.
- The leader schedule of epoch `e+1` is computed at the start of epoch
  `e`, using the location stored in the vote account at that moment,
  the same moment the schedule's stake weights are read.
  `UpdateLocation` MAY be submitted at any time and any number of
  times, but only the value visible at a schedule computation counts:
  a change submitted during epoch `e-1` is first reflected in the
  leader schedule of epoch `e+1`, with the same latency as a stake
  delegation. An already computed schedule is never adapted; within an
  epoch, every validator's location is fixed.
- The new field adds 12 bytes to the vote state (three i32).

VoteStateV5 is serialized like VoteStateV4 (SIMD-0185), with the
version discriminant 4 (u32, little-endian) in the first 4 bytes and
the position appended after `last_timestamp` as the final field:
x (i32), y (i32), z (i32), little-endian, 12 bytes. Because vote state
contains variable-length fields (votes, authorized voters, epoch
credits), fields do not have fixed offsets. The maximum serialized size
of VoteStateV5 fits comfortably within the existing 3762-byte vote
account allocation, which SIMD-0185 deliberately kept oversized to
leave room for future fields. No resize is required; new vote accounts
continue to be created with 3762 bytes.

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
  1 [signer]    validator identity
```

Errors:

- vote account not owned by the vote program: `InvalidAccountOwner`
- account 1 is not the validator identity: `MissingRequiredSignature`

Before feature `<feature-id>` is activated, the vote program MUST
reject discriminant 20 with `InvalidInstructionData`.

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
| Consensus                       | Leader schedule uses dist2          |
| Gossip                          | None                                |
| Turbine                         | None                                |
| Snapshots                       | None                                |
| On-Chain Core BPF Programs      | None                                |

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
geo-aware leader schedule (SIMD-0XXX) is built; the primitive is also
reusable for network diagnostics and telemetry.

## Security Considerations

- **The check verifies geometry, not actual location truthfulness.** A
  validator can register any point on the surface, including one far
  from its actual machine; countering location lies is the
  responsibility of the consuming leader schedule design.
- **Authenticity and replay protection** come from the existing vote
  account machinery: `UpdateLocation` is an ordinary signed
  transaction, and only the validator identity can change the stored
  location.
- **Precision is intentionally coarse.** Meter-level coordinates
  reveal no more than existing gossip IP addresses already do; a
  validator MAY round its registration to city-level precision, the
  schedule outcome is essentially unchanged.
- **Determinism.** All validation and distance computation is exact
  integer arithmetic with specified widths; there is no
  implementation-defined behavior to diverge on.

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
protocol effect. No other existing behavior changes.

## Conformance

A correct implementation MUST accept the four test vectors above with
exactly the listed `q` deviations, MUST reject the origin `(0, 0, 0)`,
and MUST reproduce the two `dist2` values exactly.
