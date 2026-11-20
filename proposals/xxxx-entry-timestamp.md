---
simd: 'XXXX'
title: Entry timestamp
authors:
  - albert is great
category: Standard/Meta
type: Core
created: 2026-11-24
feature: TODO
---

## Summary

Include intra-slot timestamps in block entry metadata.

Expose these via a new syscall, `u64 sol_gettime(ClockSource::ENTRY)`.

## Motivation

Provide on-chain markets with a clock source that is finer grained than
the slot duration.

## Dependencies *(Optional)*

This proposal depends on the following previously accepted proposals:

- **[SIMD-0326]: Alpenglow**
  Frees up the `hash_cnt` field in the entry header

## Detailed Design

### Entry timestamp

The entry timestamp is a nanosecond-precision timestamp that the leader
attaches to each block entry it creates.

The timestamp uses the unix epoch, meaning its Y2K expiration date
happens in the year 2554.

### Entry format change

An entry (microblock) in a Solana block previously had this shape.

| Offset | Type           | Name       |
|--------|----------------|------------|
| `0x00` | `u64`          | `hash_cnt` |
| `0x08` | `byte[32]`     | `hash`     |
| `0x28` | `u64`          | `txn_cnt`  |
| `0x30` | `signed txn[]` | `txns`     |

Since SIMD-0326, `hash_cnt == 1` is an enforced block validity rule.
SIMD-XXXX lifts this rule and permits an arbitrary `hash_cnt` value
without changing block hashing.

The field `hash_cnt` is then renamed to `entry_ts`, which will carry
the entry timestamp.

### `sol_gettime` syscall

The `sol_gettime` syscall is introduced (hash ID `0x2a6e819b`).

The first parameter (`r1`) must be set to `1`, or the program faults
with ERROR CODE TO BE SPECIFIED. All other registers are ignored.

`sol_gettime(1)` charges 3 CU including the syscall instruction.

The timestamp is returned via `r0`.

Compilers *should* emit the following instruction pattern to allow for
op-fusion (syscall instruction immediately preceded by mov-immediate).

```
mov64 r1, 1
syscall 0x2a6e819b
```

ABI definition:

```rs
enum ClockSource(u64) {
    ENTRY = 1,
}

fn sol_gettime(source: ClockSource) -> u64;
```

Syscall handler pseudocode:

```rs
fn sol_gettime(&mut self, source: ClockSource) -> u64 {
    self.consume_cu(2);
    return match source {
        ClockSource::ENTRY => self.context.entry_timestamp
        _ => vm_fault!(TODO ERROR CODE),
    };
}
```

### Feature Gate

The `entry_timestamp` feature gate activates the following changes:
- the `hash_cnt == 1` block validation rule is removed
- the `sol_gettime` syscall becomes available for new program
  deployments

### Edge Cases

**Clock synchronization**

A misconfigured validator without a system time clock source will post
incorrect timestamps, and a malicious node can include arbitrary
timestamps.

**Clock manipulation**

The clock source is untrusted.

An untrusted timestamp is deemed preferable over not providing an intra
block clock source at all. The vast majority of stake is honest and
reacts to social consensus, e.g. dashboards exposing validators who
manipulate semi-trusted data sources.

Trading infrastructure on mainnet has been able to protect itself
against comparable manipulation attacks (e.g. reordering of oracle
updates) via automated risk management, and is bounded by slot time.

**Precision**

The number of transactions per entries is implementation defined.

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                              |
|---------------------------------|-------------------------------------|
| Transaction Execution (Runtime) | Plumb entry data to VM              |
| Virtual Machine                 | New syscall                         |
| Block Packing                   | Now frequently pulls system time    |
| On-Chain Core BPF Programs      | New syscall                         |

## Alternatives Considered

What alternative designs were considered and what pros/cons does this feature
have relative to them?

## Impact

No impact to existing programs or validators.

New program deployments can choose to query this new clock source via
a syscall.

## Security Considerations

Frequent nanosecond accurate timestamps create a timing side channel
that is precise enough to exfiltrate microarchitectural CPU state. This
can put leaders at risk of Spectre-type attacks. Validators *may*
mitigate this by setting the 13 least significant bits to zero (8µs
precision).

## Conformance

TODO PROVIDE SOLFUZZ EXTENSION FOR INSTRUCTION CONTEXT ENTRY TS

TODO PROVIDE SOLFUZZ INSTRUCTION TEST VECTORS (WITH CU CHECKED)

TODO PROVIDE SOLFUZZ SHRED VALIDATION TEST VECTORS
