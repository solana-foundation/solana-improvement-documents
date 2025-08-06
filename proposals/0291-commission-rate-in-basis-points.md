---
simd: '0291'
title: Commission Rate in Basis Points
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2025-05-29
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Allow validators to update their inflations reward commission rate with basis
points and update commission calculation.

## Motivation

Validators should have more fine grained control over commission values beyond
integer percentage values.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0185]: Vote Account v4**

    Introduces version 4 of the vote account state, updating the commission
    field to use basis points for greater precision.

- **[SIMD-0249]: Delay Commission Updates**

    Simplifies commission update rules so that the `UpdateCommissionBps`
    instruction will not need to restrict updates in any way.

[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/pull/185
[SIMD-0249]: https://github.com/solana-foundation/solana-improvement-documents/pull/249

## New Terminology

NA

## Detailed Design

### Commission Calculation

After the adoption of [SIMD-0185], commission rates will begin to be stored in
basis points but they could still only be set in whole integer percentages. As a
result, all stored values were multiples of 100 basis points. As a result, the
actual commission calculation logic remained unchanged: the system continued to
divide by 100 and use whole integer percentage values.

This proposal introduces support for setting commission rates with full basis
point precision (e.g., 1,234 basis points = 12.34%). Because commission values
are no longer limited to clean multiples of 100, the calculation logic must now
operate directly on basis point values. To compute the inflation reward
commission while avoiding overflow, first convert the pending inflation reward
to a `u128` integer. Then multiply it by the lesser of the vote account’s
commission rate or the maximum of `10,000` basis points. Lastly, divide by
`10,000` using integer division and discard the remainder.

To calculate the portion of the reward that goes to delegated stake, again
convert to `u128`. Multiply by the greater of `0` or `10,000` minus the vote
account’s commission rate. Lastly, divide by `10,000` and discard the remainder.

### Vote Program

```rust
pub enum VoteInstruction {
    /// # Account references
    ///   0. `[WRITE]` Vote account to be updated with the new commission
    ///   1. `[SIGNER]` Withdraw authority
    UpdateCommissionBps { // 17u32
        commission_bps: u16,
        kind: CommissionKind,
    },
}

#[repr(u8)]
pub enum CommissionKind {
    InflationRewards = 0,
    BlockRevenue = 1,
}
```

#### UpdateCommissionBps

A new instruction for setting a commission rate denominated in basis points will
be added to the vote program with the enum discriminant value of `17u32` little
endian encoded in the first 4 bytes.

Perform the following checks:

- If the number of account inputs is less than 1, return
`InstructionError::NotEnoughAccountKeys`
- If the vote account (index `0`) fails to deserialize, return
`InstructionError::InvalidAccountData`
- If the vote account's authorized withdrawer is not an account input for the
instruction or is not a signer, return
`InstructionError::MissingRequiredSignature`

Update the corresponding field for the specified commission kind:

- `CommissionKind::InflationRewards`: update the
`inflation_rewards_commission_bps` field
- `CommissionKind::BlockRevenue`: return
`InstructionError::InvalidInstructionData`

Note that the commission rate is allowed to be set and stored as any `u16` value
but as detailed above, it will capped at 10,000 during the actual commission
calculation.

## Alternatives Considered

NA

## Impact

Validators will be able to set inflation rewards commission rates in basis
points. 

## Security Considerations

NA

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

A feature gate will be used to simultaneously update the vote program to support
commission rates in basis points as well as update the runtime's commission
calculations at epoch boundaries.
