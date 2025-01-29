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
    this instruction is the same as the current SPL Token-2022 instruction. There
    could be economic consequences depending on the amount of "unbricked" SOL
    &mdash; the total amount of SOL that could be freed up this way has not been
    calculated.
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

## Impact

The main impact is freeing up block CUs, allowing more transactions to be packed
in a block; dapp developers benefit since interacting with the Token program
will consume significantly less CUs.

Below is a sample of the CUs efficiency gained by `p-token` compared to the
current SPL Token program. In bracket is the percentage of CUs used in relation
to the current SPL Token consumption &mdash; the lower the percentage, the
better the gains in CUs consumption.

| Instruction | CU (`p-token`) | CU (`p-token`) + logging |CU (`spl-token`) |
|-------------|----------------|--------------------------|-----------------| 
| `InitializeMint`           | 118 (4%)  | 221 (7%)    | 2967             |
| `InitializeAccount`        | 170 (4%)  | 273 (6%)    | 4527             |
| `InitializeMultisig`       | 239 (8%)  | 348 (12%)   | 2973             |
| `Transfer`                 | 146 (3%)  | 249 (5%)    | 4645             |
| `Approve`                  | 150 (5%)  | 268 (9%)    | 2904             |
| `Revoke`                   | 124 (5%)  | 237 (9%)    | 2677             |
| `SetAuthority`             | 159 (5%)  | 261 (8%)    | 3167             |
| `MintTo`                   | 144 (3%)  | 247 (5%)    | 4538             |
| `Burn`                     | 202 (4%)  | 316 (7%)    | 4753             |
| `CloseAccount`             | 152 (5%)  | 255 (9%)    | 2916             |
| `FreezeAccount`            | 170 (4%)  | 287 (7%)    | 4265             |
| `ThawAccount`              | 169 (4%)  | 283 (7%)    | 4267             |

## Security Considerations

`p-token` must be guaranteed to follow the same instructions and accounts
layout, as well as have the same behaviour than the current Token
implementation.

Any potential risk will be mitigated by extensive fixture testing (_status_:
[completed](https://github.com/solana-program/token/blob/main/.github/workflows/main.yml#L284-L313)),
formal verification (_status_: started) and audits (_status_: scheduled). Since
there are potentially huge economic consequences of this change, the feature
will be put to a validator vote.

The replacement of the program requires breaking consensus on a non-native
program. However, this has been done in the past many times for SPL Token to fix
bugs and add new features.

## Drawbacks

N/A.

## Backwards Compatibility

Fully backwards compatible, no changes are required for users of the program.
````
