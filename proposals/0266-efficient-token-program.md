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

Apart from the original SPL Token instructions, this proposal adds three
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
    Token program. This is a new instruction that can execute a variable number of
    Token instructions in a single invocation of the Token program. Therefore, the
    base CPI invoke units (currently `1000` CU) are only consumed once, instead of
    for each CPI instruction – this significantly improves the CUs required to
    perform multiple Token instructions in a CPI context. Almost every DeFi protocol
    on Solana performs multiple CPIs to the Token program in one instruction. For
    example, an AMM performs two transfers during swap, or transfers tokens and
    mints others during an LP deposit. Programs can use the batch instruction for
    even more CU gains.

3. [`unwrap_lamports`](https://github.com/solana-program/token/pull/87)
    (instruction discriminator `45`): allows transferring out lamports from native
    SOL token accounts directly to any destination account. This eliminates the
    need for creating temporary native token accounts for the recipient. The
    instruction supports transferring a specific amount or the entire amount of
    the account.

Note that `withdraw_excess_lamports` discriminator matches the value used in SPL
Token-2022, while `batch` and `unwrap_lamports` have discriminator values that
are not used in either SPL Token nor Token-2022.

The program will be loaded into an account
(`ptokNfvuU7terQ2r2452RzVXB3o4GT33yPWo1fUkkZ2`) prior to enabling the feature
gate that triggers the replacement.

When the feature gate `ptokSWRqZz5u2xdqMdstkMKpFurauUpVen7TZXgDpkQ` is enabled,
the runtime needs to:

- Replace `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` program with
  `ptokNfvuU7terQ2r2452RzVXB3o4GT33yPWo1fUkkZ2` using the Upgradable Loader `v3`.

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
Program Tokenkeg...623VQ5DA consumed 192 of 200000 compute units
Program Tokenkeg...623VQ5DA success
```

Logging the instruction name consumes more than `100` compute units, which in
some cases reprensents almost the same amount required to execute the
instruction. Since these logs are not necessarily very reliable &mdash; e.g.,
they can be truncated, you could "log-inject" matching messages that can
"confuse" parsers &mdash; we propose to omit logging.

Sample CU consumption for `p-token` without logs:
| Instruction          | `p-token` *- logs* (CU) | `p-token` (CU) |
|----------------------|-----------------------|----------------------|
| `InitializeMint`     | 105                   | 214                  |
| `InitializeAccount`  | 154                   | 264                  |
| `Transfer`           |  76                   | 186                  |
| `TransferChecked`    | 105                   | 218                  |
| `MintTo`             | 123                   | 231                  |
| `Burn`               | 133                   | 237                  |
| `CloseAccount`       | 125                   | 229                  |

## Impact

The main impact is freeing up block CUs, allowing more transactions to be packed
in a block; dapp developers benefit since interacting with the Token program
will consume significantly less CUs.

Below is a sample of the CUs efficiency gained by `p-token` compared to the
current SPL Token program, including the percentage of CUs used in relation
to the current SPL Token consumption &mdash; the lower the percentage, the
better the gains in CUs consumption.

| Instruction                  | spl token | p-token | % of spl-token |
| ---------------------------- | --------- | ------- | -------------- |
| `Approve`                    | 2904      | 124     | 4.2%           |
| `ApproveChecked`             | 4458      | 164     | 3.6%           |
| `Burn`                       | 4753      | 126     | 2.6%           |
| `BurnChecked`                | 4754      | 129     | 2.7%           |
| `CloseAccount`               | 2916      | 120     | 4.1%           |
| `FreezeAccount`              | 4265      | 146     | 3.4%           |
| `InitializeAccount`          | 4527      | 154     | 3.4%           |
| `InitializeAccount2`         | 4388      | 171     | 3.8%           |
| `InitializeAccount3`         | 4240      | 248     | 5.8%           |
| `InitializeImmutableOwner`   | 1404      | 38      | 2.7%           |
| `InitializeMint`             | 2967      | 105     | 3.5%           |
| `InitializeMint2`            | 2827      | 228     | 8.0%           |
| `InitializeMultisig`         | 2973      | 193     | 6.4%           |
| `InitializeMultisig2`        | 2826      | 318     | 11.2%          |
| `MintTo`                     | 4538      | 119     | 2.6%           |
| `MintToChecked`              | 4545      | 169     | 3.7%           |
| `Revoke`                     | 2677      | 97      | 3.6%           |
| `SetAuthority`               | 3167      | 123     | 3.8%           |
| `SyncNative`                 | 3045      | 61      | 2.0%           |
| `ThawAccount`                | 4267      | 142     | 3.3%           |
| `Transfer`                   | 4645      | 76      | 1.6%           |
| `TransferChecked`            | 6200      | 105     | 1.6%           |

Considering the usage distribution of instructions (shown below), migrating
to p-token will significantly reduce the block CUs currently consumed by the
token program.

| Instruction                 | Usage (%) |
| --------------------------- | --------- |
| `TransferChecked`           | `36.33%`  |
| `Transfer`                  | `13.22%`  |
| `CloseAccount`              | `12.23%`  |
| `InitializeAccount3`        |  `9.98%`  |
| `InitializeImmutableOwner`  |  `9.78%`  |
| `SyncNative`                |  `4.53%`  |
| `InitializeAccount`         |  `2.58%`  |

Other instructions account for less than `1%` each.

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

- ✅ **[COMPLETED]** Audits
  - [Neodyme Audit (2025-06-12)](https://github.com/anza-xyz/security-audits/blob/master/spl/NeodymePTokenPinocchioAudit-2025-06-12.pdf)
  - [Zellic Audit (2025-06-30)](https://github.com/anza-xyz/security-audits/blob/master/spl/ZellicPTokenPinocchioAudit-2025-06-30.pdf)
  - [Zellic Audit (2025-10-13)](https://github.com/anza-xyz/security-audits/blob/master/spl/ZellicPTokenAudit-2025-10-13.pdf)

- ⏳ *[IN PROGRESS]* Formal Verification

Since there are potentially huge economic consequences of this change, the feature
will be put to a validator vote.

The replacement of the program requires breaking consensus on a non-native
program. However, this has been done in the past many times for SPL Token to fix
bugs and add new features.

## Drawbacks

N/A.

## Backwards Compatibility

Fully backwards compatible, no changes are required for users of the program.
Note that we are proposing omitting instruction name logs
(e.g., "Instruction: &#60;name&#62;") .
````
