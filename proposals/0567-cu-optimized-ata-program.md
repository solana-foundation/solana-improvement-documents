---
simd: '0567'
title: "p-ATA: CU-optimized ATA Program"
authors:
  - Gabe R. (Anza)
  - Peter K. (Ergonia)
category: Standard
type: Core
status: Review
created: 2026-06-17
feature: TBD
---

## Summary

This proposal outlines a plan to replace the current Associated Token Account
program (`ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL`) with a Pinocchio-based,
CU-optimized implementation (`p-ATA`) at the same program address.

The replacement is a drop-in implementation for the existing `Create`,
`CreateIdempotent`, and `RecoverNested` instructions with the same ABI. It also
adds a new `CreateWithArgs` instruction for callers that can provide optional
optimization hints.

## Motivation

ATA creation is one of the most frequently executed instructions on Solana. This
program governs the standard that derives canonical token addresses and handles
the CPIs to create the token accounts. This makes the ATA program a
high-leverage target for compute reduction.

On a 30k confirmed-block stratified mainnet sample:

- ATA appears in 11.85% of transactions (31.3% non-vote only)
- ATA share of total CU usage: 13.28%
- Top invoked programs, ranked by instruction count:
    1. spl token (p-token already live),
    2. compute budget (Transaction V1 makes this ix unnecessary)
    3. vote (removed in Alpenglow)
    4. system (a runtime built-in)
    5. ATA (the next best candidate to pinocchio-ize)

[P-Token][SIMD-0266] was a strong validation of the pinocchio philosophy:
zero-copy + no-std + no-alloc + low deps. The CU usage of the token program
effectively collapsed (-93%) with the usage as active as before. No userflow
changes, but simply meaningfully freed block capacity on the network. The same
results can be had with the ATA program.

## Impact

The average CU reduction (weighted by instruction frequency) of `p-ATA`
instructions is 81% against the legacy ATA program baseline. Given ATA's share
of total CU usage, we can expect a cluster-wide CU reduction of ~10-11%.

| Case                    | Share of use | Legacy | p-ATA | Reduction |
|-------------------------|-------------:|-------:|------:|----------:|
| idempotent new SPL      |       39.86% | 22,940 | 4,171 |    -81.8% |
| idempotent existing SPL |       30.32% |  3,710 |   530 |    -85.7% |
| idempotent existing T22 |       18.60% |  8,210 | 1,616 |    -80.3% |
| create SPL              |        5.65% | 18,433 | 3,085 |    -83.3% |
| idempotent new T22      |        3.65% | 15,474 | 5,496 |    -64.5% |
| create T22              |        1.91% | 13,967 | 5,132 |    -63.3% |
| recover_nested SPL/SPL  |        0.00% | 26,806 | 5,152 |    -80.8% |

`CreateWithArgs` is a new proposed instruction where callers can optionally
provide the bump, account length, and rent sysvar. The full-hint CU
measurements are:

| Case                     |  Base | WithArgs |  Delta |
|--------------------------|------:|---------:|-------:|
| idempotent new SPL       | 4,171 |    3,925 |   -246 |
| idempotent existing SPL  |   530 |      387 |   -143 |
| idempotent existing T22  | 1,616 |      387 | -1,229 |
| create SPL               | 3,085 |    2,831 |   -254 |
| idempotent new T22       | 5,496 |    5,684 |   +188 |
| create T22               | 5,132 |    5,314 |   +182 |
| create prefunded SPL     | 3,085 |    2,831 |   -254 |
| create prefunded T22     | 5,132 |    5,314 |   +182 |
| create T22 extended mint | 6,674 |    6,130 |   -544 |

More details on caller guidance and tradeoffs for this instruction below.

## New Terminology

n/a

## Detailed Design

`p-ATA` is a no-std [Pinocchio] program. It avoids heap allocation, uses
zero-copy instruction/account parsing, and minimizes dependencies. The
[implementation] (tracked in [associated-token-account#196]) splits into a
public `interface` crate (instructions, PDA derivation, errors) and the on-chain
`program` crate. It maintains strict compatibility with the existing ATA
program:

- Unchanged Program ID: `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL`
- Backward compatible instruction interface: discriminants, required accounts,
  and signer statuses are identical
- Identical address and bump derivation
- Same semantics with error codes and state transitions

### New instruction: `CreateWithArgs`

`p-ATA` adds a new instruction that carries optional optimization hints. Each
hint reserves `0` as its absent value, so it occupies a fixed slot in the wire
format with no separate presence flag. The serialized payload is exactly seven
bytes:

| Byte range | Field               | Description                           |
|------------|---------------------|---------------------------------------|
| `0`        | discriminator       | `3`                                   |
| `1`        | mode                | `0` = always create, `1` = idempotent |
| `2`        | bump hint           | Bump for the ATA address              |
| `3..7`     | account length hint | unsigned 32-bit little-endian integer |

For bump and account length hint, `0` means absent.

The account list is the same as `Create` / `CreateIdempotent`, with one optional
trailing rent sysvar:

| Index | Role                  | Description                                |
|-------|-----------------------|--------------------------------------------|
| 0     | `[writeable, signer]` | Funding account                            |
| 1     | `[writeable]`         | Associated token account address to create |
| 2     | `[]`                  | Wallet address for the ATA                 |
| 3     | `[]`                  | Token mint                                 |
| 4     | `[]`                  | System program                             |
| 5     | `[]`                  | SPL Token or Token-2022 program            |
| 6     | `[]` optional         | Rent sysvar (NEW)                          |

#### Caller guidance

All optimization params (bump, account length, & rent sysvar account) are optional
with trade offs depending on the caller's expected case.

<!-- markdownlint-disable MD013 -->
| Case                     | No hints | Bump only | Len only | Rent only | Full hints |
|--------------------------|---------:|----------:|---------:|----------:|-----------:|
| create SPL               |      +14 |      -129 |      +15 |      -113 |       -254 |
| create T22               |      +13 |      -130 |       +9 |      +328 |       +182 |
| create T22 extended mint |      +14 |      -129 |     -717 |      +329 |       -544 |
| idempotent new SPL       |      +19 |      -121 |      +20 |      -108 |       -246 |
| idempotent new T22       |      +16 |      -124 |      +12 |      +331 |       +188 |
| idempotent existing SPL  |      +21 |      -153 |      +22 |       +30 |       -143 |
| idempotent existing T22  |      +21 |    -1,239 |      +22 |       +30 |     -1,229 |
<!-- markdownlint-restore -->

- If the caller provides no optimization params, use `Create`/`CreateIdempotent`
  instead.
- If the call is idempotent and the ATA is likely to already exist:
  - If the caller knows the correct bump, use `CreateWithArgs` with the bump.
  - Otherwise use `CreateIdempotent`.
- If the token program is SPL Token:
  - Ignore `account_len`, because SPL Token account length is fixed.
  - Include the correct bump when available.
  - Include the rent sysvar when account creation is likely.
  - Omit the rent sysvar when optimizing for mostly-existing idempotent calls.
- If the token program is Token-2022:
  - Include the correct bump & `account_len` when known.
  - Omit the rent sysvar for creation paths until the Token-2022 program has
    zero copy account parsing. After this, it will be beneficial (-390 CUs). At
    the moment, it's a penalty.

### `RecoverNested`

`RecoverNested` keeps its existing discriminator (`2`), but `p-ATA` extends it
beyond what the legacy program supports:

- The nested mint may use a different token program than the owner mint
- The wallet may be a token multisig
- The nested mint may carry a transfer hook

These additions are backward compatible. The new accounts are optional and
trailing, so a caller passing the legacy account layout gets identical behavior.

The instruction supports the following account layout:

<!-- markdownlint-disable MD013 -->
| Index | Role                                    | Description                             |
|-------|-----------------------------------------|-----------------------------------------|
| 0     | `[writeable]`                           | Nested ATA, owned by the owner ATA      |
| 1     | `[]`                                    | Nested token mint                       |
| 2     | `[writeable]`                           | Wallet's ATA for the nested mint        |
| 3     | `[]`                                    | Owner ATA                               |
| 4     | `[]`                                    | Owner token mint                        |
| 5     | `[writeable, signer (if not multisig)]` | Wallet or token multisig wallet (NEW)   |
| 6     | `[]`                                    | Token program for the owner mint        |
| 7     | `[]` optional                           | Token program for the nested mint (NEW) |
| `8..` | `[signer]` optional                     | Multisig signer accounts (NEW)          |
<!-- markdownlint-restore -->

The nested token program account is optional when it is the same as the owner
token program and the wallet signs directly. It is required when the nested mint
uses a different token program, and it is also required when the wallet is a
token multisig so that trailing signer accounts are unambiguous.

### Program migration

Since the current mainnet ATA program is a Loader v2 program, the migration will
follow [SIMD-0418]. A Loader v3 buffer containing the verified `p-ATA` ELF will
be deployed before activation. When the feature activates, the runtime creates
the derived program data account, rewrites the existing ATA program account as a
Loader v3 program account with no upgrade authority, and closes the source
buffer.

#### Reproducible build artifact

The migration configuration must specify the source Loader v3 buffer account and
expected verified build hash. The `p-ATA` ELF can be reproduced from the pinned
implementation commit:

```sh
git clone https://github.com/solana-program/associated-token-account.git
cd associated-token-account
git checkout daba656acc2949d3bf7903c69bd3357192d0dfe0
make build-sbf-pinocchio-program

# The generated ELF is located at:
# target/deploy/pinocchio_associated_token_account_program.so

solana-verify get-executable-hash \
  target/deploy/pinocchio_associated_token_account_program.so
```

The expected verified build hash is:

```text
c667326d5d7afbe7e9996e6d05259ce6fca36b310b5cdf7eb6380f2f1d953d4f
```

#### Program availability

During the migration slot in which feature activation is applied, the ATA program
will not be invocable. Loader v3 programs become effective one slot after the
deployment slot.

## Dependencies

- **[SIMD-0312]: CreateAccountAllowPrefund** Recently activated on mainnet.
  Preserves prefunded-ATA support while avoiding the legacy `Transfer` +
  `Allocate` + `Assign` sequence.

- **[SIMD-0418]: Loader v2 to v3 Program Migrations**. `p-ATA` will re-use the
  same pattern exercised by `p-token` for the program upgrade.

- **[SIMD-0449]: Direct Account Pointers** Pending mainnet activation. `p-ATA`
  will save CUs by using the new direct-account-pointer entrypoint.

## Alternatives Considered

An alternative is to extend the existing `Create` and `CreateIdempotent`
instructions with the optional hints instead of adding `CreateWithArgs`. We
rejected this. At p-token's launch, some callers turned out to be passing extra
accounts that the legacy program silently ignored, and the stricter p-token
rejected them, which broke those callers. Overloading `Create` and
`CreateIdempotent` to read a trailing rent sysvar or extra hint bytes would
reinterpret data those callers already send and risk the same breakage.

## Security Considerations

- **Comprehensive Testing**: All existing test fixtures will be ported and
  passed. A differential testing framework will execute instructions against
  both program versions and assert that the resulting state and status are
  identical.
- **Fuzzing**: The program will be heavily fuzzed by replaying historical
  mainnet transactions. Working with Firedancer to source fixtures.
- **External Security Audit**: The final implementation will undergo at least
  one comprehensive external security audit.
- **Formal Verification**: Employ formal methods to prove that p-ATA
  demonstrates semantic equivalence with the legacy implementation.

[SIMD-0266]: ./0266-efficient-token-program.md
[SIMD-0418]: ./0418-enable-loader-v2-to-v3-program-migrations.md
[SIMD-0449]: ./0449-direct-account-pointers-in-program-input.md
[Pinocchio]: https://github.com/anza-xyz/pinocchio
[implementation]: https://github.com/solana-program/associated-token-account/tree/main/pinocchio
[associated-token-account#196]: https://github.com/solana-program/associated-token-account/issues/196
