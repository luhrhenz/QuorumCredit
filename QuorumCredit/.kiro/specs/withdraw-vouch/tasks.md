# Implementation Plan: withdraw-vouch

## Overview

Add `withdraw_vouch(voucher, borrower)` to `QuorumCreditContract` in `src/lib.rs`. The function removes a `VouchRecord` from persistent storage and returns the exact stake to the voucher, guarded by auth and a pre-loan check.

## Tasks

- [x] 1. Implement `withdraw_vouch` function
  - [x] 1.1 Add `withdraw_vouch` method to `QuorumCreditContractClient` in `src/lib.rs`
    - Call `voucher.require_auth()` as the first statement
    - Assert `DataKey::Loan(borrower.clone())` is absent in persistent storage, panicking with `"loan already active"` if present
    - Load `Vec<VouchRecord>` from `DataKey::Vouches(borrower.clone())`, panicking with `"vouch not found"` if absent
    - Find the index of the `VouchRecord` whose `voucher` field matches the argument; panic with `"vouch not found"` if no match
    - Record the matched `stake` value and remove the entry from the vector
    - If the vector is now empty, remove `DataKey::Vouches(borrower)` from persistent storage; otherwise write the updated vector back
    - Transfer `stake` stroops from the contract address to `voucher` via the token client
    - _Requirements: 1.1, 1.2, 2.1, 2.2, 3.1, 3.2, 4.1, 4.2, 5.1, 5.2, 5.3_

  - [ ]* 1.2 Write property test: auth is required
    - **Property: Voucher authorization is enforced**
    - Verify that calling `withdraw_vouch` without mocking auth for the voucher panics or is rejected
    - **Validates: Requirements 1.1, 1.2**

- [x] 2. Write unit tests for `withdraw_vouch`
  - [x] 2.1 Test: happy path — stake returned and vouch record removed
    - Call `vouch`, then `withdraw_vouch`; assert voucher token balance equals pre-vouch balance and `get_vouches` returns an empty list
    - _Requirements: 4.1, 4.2, 5.1, 5.2_

  - [ ]* 2.2 Write property test: round-trip balance consistency
    - **Property: vouch followed by withdraw_vouch restores exact pre-vouch balance**
    - For any valid stake amount, `balance_after == balance_before`
    - **Validates: Requirements 6.1**

  - [ ]* 2.3 Write property test: round-trip state consistency
    - **Property: vouch followed by withdraw_vouch leaves no VouchRecord for that voucher**
    - `get_vouches` result must not contain a record with the withdrawn voucher address
    - **Validates: Requirements 6.2**

  - [x] 2.4 Test: panics with `"loan already active"` when a LoanRecord exists
    - Call `vouch`, `request_loan`, then `withdraw_vouch`; assert the call panics with `"loan already active"`
    - _Requirements: 2.1_

  - [x] 2.5 Test: panics with `"vouch not found"` when no matching VouchRecord exists
    - Call `withdraw_vouch` for a voucher/borrower pair that was never vouched; assert panic with `"vouch not found"`
    - _Requirements: 3.1_

  - [x] 2.6 Test: only the target VouchRecord is removed when multiple vouchers exist
    - Call `vouch` twice with different vouchers, then `withdraw_vouch` for one; assert the other VouchRecord is still present in `get_vouches`
    - _Requirements: 5.3_

  - [x] 2.7 Test: `Vouches` key is removed from storage when last vouch is withdrawn
    - Call `vouch` once, then `withdraw_vouch`; assert `get_vouches` returns an empty vec (key absent)
    - _Requirements: 5.2_

- [x] 3. Final checkpoint
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- All tests live in the existing `#[cfg(test)] mod tests` block in `src/lib.rs`
- The `setup` helper already mints tokens and initialises the contract — reuse it in all new tests
- No new storage keys, data types, or dependencies are needed
