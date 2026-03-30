# Requirements Document

## Introduction

This feature adds an on-chain credit score to the QuorumCredit lending protocol. Each borrower accumulates a credit history derived from their repayment and default events. The score is computed deterministically from `repayment_count` and `default_count` fields stored in persistent contract storage, and is exposed as a read-only view function `get_credit_score(borrower)`. The goal is to give lenders, vouchers, and integrators a single numeric signal of borrower trustworthiness without relying on off-chain data.

## Glossary

- **Contract**: The `QuorumCreditContract` Soroban smart contract.
- **Borrower**: An `Address` that has requested at least one loan through the Contract.
- **CreditScore**: A `u32` value computed from a Borrower's on-chain repayment history. Higher is better.
- **RepaymentCount**: A `u32` counter stored under `DataKey::RepaymentCount(borrower)` that records the total number of loans fully repaid by a Borrower.
- **DefaultCount**: A `u32` counter stored under `DataKey::DefaultCount(borrower)` that records the total number of loans that ended in a slash or expiry default for a Borrower.
- **Slash**: The governance action that marks a loan as defaulted and penalises vouchers.
- **ScoreFormula**: The deterministic function `CreditScore = RepaymentCount * 10 - DefaultCount * 20`, floored at 0.

---

## Requirements

### Requirement 1: Track Repayment Count

**User Story:** As a voucher, I want the Contract to record every successful full repayment so that I can assess a Borrower's reliability before staking.

#### Acceptance Criteria

1. WHEN a Borrower fully repays an active loan, THE Contract SHALL increment `RepaymentCount` for that Borrower by exactly 1.
2. THE Contract SHALL initialise `RepaymentCount` to 0 for any Borrower that has never repaid a loan.
3. WHEN `repayment_count(borrower)` is called, THE Contract SHALL return the current `RepaymentCount` for that Borrower.
4. WHILE a loan is only partially repaid, THE Contract SHALL NOT increment `RepaymentCount`.

---

### Requirement 2: Track Default Count

**User Story:** As a voucher, I want the Contract to record every default event so that I can avoid staking on repeat defaulters.

#### Acceptance Criteria

1. WHEN a Borrower's loan is slashed via the governance slash path, THE Contract SHALL increment `DefaultCount` for that Borrower by exactly 1.
2. THE Contract SHALL initialise `DefaultCount` to 0 for any Borrower that has never defaulted.
3. WHEN `default_count(borrower)` is called, THE Contract SHALL return the current `DefaultCount` for that Borrower.

---

### Requirement 3: Compute and Expose Credit Score

**User Story:** As an integrator, I want a single numeric credit score per Borrower so that I can rank borrowers without reading multiple storage keys.

#### Acceptance Criteria

1. THE Contract SHALL expose a read-only function `get_credit_score(borrower: Address) -> u32`.
2. WHEN `get_credit_score(borrower)` is called, THE Contract SHALL return `(RepaymentCount * 10).saturating_sub(DefaultCount * 20)` as a `u32`.
3. WHEN a Borrower has no repayment or default history, THE Contract SHALL return a `CreditScore` of 0.
4. IF `DefaultCount * 20` exceeds `RepaymentCount * 10`, THEN THE Contract SHALL return 0 rather than underflowing.
5. WHEN `get_credit_score(borrower)` is called, THE Contract SHALL NOT modify any contract state.

---

### Requirement 4: Score Consistency Invariants

**User Story:** As a protocol auditor, I want the credit score to remain consistent with the underlying counters so that the score cannot be manipulated independently.

#### Acceptance Criteria

1. THE Contract SHALL ensure that for all Borrowers, `get_credit_score(borrower)` equals `(repayment_count(borrower) * 10).saturating_sub(default_count(borrower) * 20)` at all times.
2. WHEN `RepaymentCount` is incremented, THE Contract SHALL reflect the updated value in the next call to `get_credit_score`.
3. WHEN `DefaultCount` is incremented, THE Contract SHALL reflect the updated value in the next call to `get_credit_score`.

---

### Requirement 5: Test Coverage

**User Story:** As a developer, I want automated tests for the credit score feature so that regressions are caught before deployment.

#### Acceptance Criteria

1. THE test suite SHALL include a test that verifies `get_credit_score` returns 0 for a Borrower with no history.
2. THE test suite SHALL include a test that verifies `get_credit_score` increases by 10 after each successful repayment.
3. THE test suite SHALL include a test that verifies `get_credit_score` decreases by 20 after each default, floored at 0.
4. THE test suite SHALL include a test that verifies `RepaymentCount` and `DefaultCount` are incremented exactly once per qualifying event.
5. THE test suite SHALL include a property test verifying that for any sequence of repayments `r` and defaults `d`, `get_credit_score` equals `(r * 10).saturating_sub(d * 20)`.
