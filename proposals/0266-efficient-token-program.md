---
simd: "0266"
title: "p-token: Efficient Token program"
authors:
  - febo (Anza)
  - Jon Cinque (Anza)
category: Standard
type: Core
status: Review
created: 2025-03-19
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Replace the current version of SPL Token
(`TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`) program by a CU-optimized one
(`p-token`).

## Motivation

About `~10%` of block compute units is used by the Token program instructions.
Decreasing the CUs used by Token program instructions creates block space for
other transactions to be executed – i.e., less CUs consumed by Token program,
more CUs for other transactions.

As an example, if we reduce the CUs consumed by Token instructions to `1/20th`
of their current value, `10%` of block CUs utilized becomes `0.5%`, resulting in
a `9.5%` block space gain.

Additionally, there are benefits to downstream programs:

- Better composability since using the Token program instructions require less
  CUs. - Cheaper (in terms of CU) cross-program invocations.

## New Terminology

N/A.

## Detailed Design

`p-token`
([repository](https://github.com/solana-program/token/tree/main/p-token)) is a
like-for-like efficient re-implementation of the Token program. It is `no_std`
(no heap memory allocations are made in the program) and uses zero-copy access
for instruction and account data. Since it follows the same instructions and
accounts layout, it does not require any changes to client code – it works as a
drop-in replacement.

Apart from the original SPL Token instructions, this proposal adds two
additional instructions to the program:

1. [`withdraw_excess_lamports`](https://github.com/solana-program/token/blob/main/p-token/src/processor/withdraw_excess_lamports.rs)
    (instruction discriminator `38`): allow recovering "bricked" SOL from mint
    (e.g., USDC mint as `~323` SOL in excess) and multisig accounts. The logic of
    this instruction is similar to the current SPL Token-2022 instruction: the mint
    authority must be a signer (for mint accounts) or the multisig (for multisig
    accounts) to authorize the withdraw. Additionally, for mint accounts that do
    not have a mint authority set, it is possible to authorize the withdraw using
    the mint account as the signing authority &mdash; the instruction will need to
    be signed using the mint private key. Note that economic consequences may occur
    depending on the quantity of "unbricked" SOL; the total amount of SOL that could
    be freed up in this manner has yet to be calculated.

2. [`batch`](https://github.com/solana-program/token/blob/main/p-token/src/processor/batch.rs)
    (instruction discriminator `255`): enable efficient CPI interaction with the
    Token program. This is a new instruction that can execute a variable number on
    Token instructions in a single invocation of the Token program. Therefore, the
    base CPI invoke units (currently `1000` CU) are only consumed once, instead of
    for each CPI instruction – this significantly improves the CUs required to
    perform multiple Token instructions in a CPI context. Almost every DeFi protocol
    on Solana performs multiple CPIs to the Token program in one instruction. For
    example, an AMM performs two transfers during swap, or transfers tokens and
    mints others during an LP deposit. Programs can use the batch instruction for
    even more CU gains.

Note that `withdraw_excess_lamports` discriminator matches the value used in SPL
Token-2022, while `batch` has a driscriminator value that is not used in either
SPL Token nor Token-2022.

The program will be loaded into an account
(`ptokNfvuU7terQ2r2452RzVXB3o4GT33yPWo1fUkkZ2`) prior to enabling the feature
gate that triggers the replacement.

When the feature gate `ptokSWRqZz5u2xdqMdstkMKpFurauUpVen7TZXgDpkQ` is enabled,
the runtime needs to:

- Replace `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` program with
  `ptokNfvuU7terQ2r2452RzVXB3o4GT33yPWo1fUkkZ2` using the Upgradable Loader `v4`.

## Alternatives Considered

As an alternative to replace the current version of SPL Token, `p-token` could
be deployed to a new address and people can be encouraged to transition to that
program. This would hinder its adoption and benefits, since people would be very
slow to adopt the new program, if they adopt it at all.

Another point considered was to whether keep the current logs on the program or
not. Logs are limited to show the name of the instruction being executed, e.g.:

```
Program Tokenkeg...623VQ5DA invoke [1]
Program log: Instruction: Transfer
Program Tokenkeg...623VQ5DA consumed 249 of 200000 compute units
Program Tokenkeg...623VQ5DA success
```

Logging the instruction name consumes around `103` compute units, which in some
cases reprensents almost the same amount required to execute the instruction,
although removing the logs can be too disruptive to anyone relying on them.

Sample CU consumption for `p-token` without logs:
| Instruction          | `p-token` *- logs* (CU) | `p-token` (CU) |
|----------------------|-----------------------|----------------------|
| `InitializeMint`     | 118                   | 221                  |
| `InitializeAccount`  | 170                   | 273                  |
| `Transfer`           | 146                   | 249                  |
| `MintTo`             | 144                   | 247                  |
| `Burn`               | 202                   | 316                  |
| `CloseAccount`       | 152                   | 255                  |

## Impact

The main impact is freeing up block CUs, allowing more transactions to be packed
in a block; dapp developers benefit since interacting with the Token program
will consume significantly less CUs.

Below is a sample of the CUs efficiency gained by `p-token` compared to the
current SPL Token program. In bracket is the percentage of CUs used in relation
to the current SPL Token consumption &mdash; the lower the percentage, the
better the gains in CUs consumption.

| Instruction          | `p-token` (CU) | `spl-token` (CU) |
|----------------------|----------------|------------------| 
| `InitializeMint`     | 221 (7%)       | 2967             |
| `InitializeAccount`  | 273 (6%)       | 4527             |
| `InitializeMultisig` | 348 (12%)      | 2973             |
| `Transfer`           | 249 (5%)       | 4645             |
| `Approve`            | 268 (9%)       | 2904             |
| `Revoke`             | 237 (9%)       | 2677             |
| `SetAuthority`       | 261 (8%)       | 3167             |
| `MintTo`             | 247 (5%)       | 4538             |
| `Burn`               | 316 (7%)       | 4753             |
| `CloseAccount`       | 255 (9%)       | 2916             |
| `FreezeAccount`      | 287 (7%)       | 4265             |
| `ThawAccount`        | 283 (7%)       | 4267             |

## Security Considerations

`p-token` must be guaranteed to follow the same instructions and accounts
layout, as well as have the same behaviour than the current Token
implementation.

Any potential risk will be mitigated by extensive testing and auditing:

- ✅ **[COMPLETED]** Existing SPL Token test [fixtures](https://github.com/solana-program/token/blob/main/.github/workflows/main.yml#L284-L313)

- ✅ **[COMPLETED]** Fuzzing using Firedancer tooling
([solfuzz_agave](https://github.com/firedancer-io/solfuzz-agave)): this
includes executing past mainnet instructions &mdash; with or without random
modifications amounting to millions of individual instructions &mdash; and
verifying that the complete program output (i.e., both the program result
and accounts' state) matches.

- ⏳ *[IN PROGRESS]* Formal Verification

- ⏳ *[IN PROGRESS]* Audits

Since there are potentially huge economic consequences of this change, the feature
will be put to a validator vote.

The replacement of the program requires breaking consensus on a non-native
program. However, this has been done in the past many times for SPL Token to fix
bugs and add new features.

## Drawbacks

N/A.

## Backwards Compatibility

Fully backwards compatible, no changes are required for users of the program.
````
