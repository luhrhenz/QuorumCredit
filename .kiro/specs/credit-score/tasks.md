# Implementation Plan: Credit Score

## Overview

Purely additive implementation: add `get_credit_score()` to `src/loan.rs` and expose it via `src/lib.rs`, then write unit and property-based tests in a new `src/credit_score_test.rs` module. No existing logic changes.

## Tasks

- [ ] 1. Add `proptest` to dev-dependencies in `Cargo.toml`
  - Append `proptest = "1"` under `[dev-dependencies]`
  - _Requirements: 5.5_

- [ ] 2. Implement `get_credit_score` in `src/loan.rs`
  - [ ] 2.1 Add the `get_credit_score(env: Env, borrower: Address) -> u32` function
    - Read `DataKey::RepaymentCount(borrower.clone())` with `unwrap_or(0)`
    - Read `DataKey::DefaultCount(borrower)` with `unwrap_or(0)`
    - Return `(repayments * 10).saturating_sub(defaults * 20)`
    - Place alongside the existing `repayment_count` and `default_count` helpers
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ] 3. Expose `get_credit_score` on `QuorumCreditContract` in `src/lib.rs`
  - [ ] 3.1 Add the public contract method delegating to `loan::get_credit_score`
    - Add `pub fn get_credit_score(env: Env, borrower: Address) -> u32` to `#[contractimpl]`
    - Place it alongside `repayment_count` and `default_count`
    - _Requirements: 3.1_

- [ ] 4. Create `src/credit_score_test.rs` with unit tests
  - [ ] 4.1 Write unit test: score is 0 for a fresh borrower with no history
    - Use `soroban_sdk::testutils` to set up a minimal env with no storage entries
    - Assert `get_credit_score` returns 0
    - _Requirements: 3.3, 5.1_
  - [ ] 4.2 Write unit test: score increases by 10 after each successful repayment
    - Manually set `DataKey::RepaymentCount` to 1, 2, 3 and assert scores 10, 20, 30
    - _Requirements: 1.1, 5.2_
  - [ ] 4.3 Write unit test: score decreases by 20 after each default, floored at 0
    - Set `DataKey::DefaultCount` to 1 with 0 repayments → assert score 0
    - Set repayments=3, defaults=2 → assert score `(30).saturating_sub(40)` = 0
    - Set repayments=5, defaults=1 → assert score 30
    - _Requirements: 2.1, 3.4, 5.3_
  - [ ] 4.4 Write unit test: `RepaymentCount` and `DefaultCount` are each incremented exactly once per qualifying event
    - Simulate a full repayment via `loan::repay` in a test env and assert `repayment_count` goes from 0 to 1
    - Simulate a slash via `governance::vote_slash` reaching quorum and assert `default_count` goes from 0 to 1
    - _Requirements: 1.1, 2.1, 5.4_

- [ ] 5. Checkpoint — ensure all unit tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 6. Add property-based tests to `src/credit_score_test.rs`
  - [ ]* 6.1 Write property test for formula fidelity (Property 1)
    - **Property 1: Score formula fidelity**
    - **Validates: Requirements 3.2, 4.1**
    - Generate arbitrary `(r: u32, d: u32)`, write them directly to storage, call `get_credit_score`, assert result equals `(r * 10).saturating_sub(d * 20)`
    - Tag: `// Feature: credit-score, Property 1: Score formula fidelity`
  - [ ]* 6.2 Write property test for zero-history score (Property 2)
    - **Property 2: Zero-history score is zero**
    - **Validates: Requirements 3.3**
    - Generate arbitrary borrower address with no storage entries, assert score == 0
    - Tag: `// Feature: credit-score, Property 2: Zero-history score is zero`
  - [ ]* 6.3 Write property test for saturation floor (Property 3)
    - **Property 3: Saturation floor — no underflow**
    - **Validates: Requirements 3.4**
    - Generate `(r, d)` where `d * 20 > r * 10` (i.e. `d > r / 2`), assert score == 0
    - Tag: `// Feature: credit-score, Property 3: Saturation floor — no underflow`
  - [ ]* 6.4 Write property test for read-only behaviour (Property 4)
    - **Property 4: Read-only — no state mutation**
    - **Validates: Requirements 3.5**
    - Snapshot `RepaymentCount` and `DefaultCount` before calling `get_credit_score`, assert both are unchanged after the call
    - Tag: `// Feature: credit-score, Property 4: Read-only — no state mutation`
  - [ ]* 6.5 Write property test for repayment delta (Property 5)
    - **Property 5: Repayment increments score by 10**
    - **Validates: Requirements 1.1, 4.2**
    - Generate arbitrary prior `(r, d)`, record score, increment `RepaymentCount` by 1, assert new score == `old_score.saturating_add(10)`
    - Tag: `// Feature: credit-score, Property 5: Repayment increments score by 10`
  - [ ]* 6.6 Write property test for default delta (Property 6)
    - **Property 6: Default decrements score by 20 (floored at 0)**
    - **Validates: Requirements 2.1, 4.3**
    - Generate arbitrary prior `(r, d)`, record score, increment `DefaultCount` by 1, assert new score == `old_score.saturating_sub(20)`
    - Tag: `// Feature: credit-score, Property 6: Default decrements score by 20 (floored at 0)`

- [ ] 7. Register `credit_score_test` module in `src/lib.rs`
  - Add `#[cfg(test)] mod credit_score_test;` alongside the other test module declarations
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [ ] 8. Final checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Property tests require `proptest = "1"` in `[dev-dependencies]` (Task 1)
- All implementation is additive — no existing functions are modified
- `saturating_sub` on `u32` guarantees the floor-at-0 behaviour without any explicit `if` branch
