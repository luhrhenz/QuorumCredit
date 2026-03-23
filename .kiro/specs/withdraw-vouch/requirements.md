# Requirements Document

## Introduction

The `withdraw_vouch` feature allows a voucher to reclaim their staked XLM from the QuorumCredit contract when the borrower they vouched for has not yet requested a loan. This gives vouchers an exit path before any loan is active, reducing the risk of capital being locked indefinitely. Once a loan is active (requested, repaid, or defaulted), the normal repay/slash flows govern stake return instead.

## Glossary

- **QuorumCredit_Contract**: The Soroban smart contract managing vouches, loans, repayments, and slashing.
- **Voucher**: An account that has staked XLM to vouch for a borrower.
- **Borrower**: An account that may request a loan backed by voucher stakes.
- **Stake**: The amount of XLM (in stroops) a voucher has deposited into the contract on behalf of a borrower.
- **VouchRecord**: The on-chain record storing a voucher's address and stake amount for a given borrower.
- **LoanRecord**: The on-chain record storing loan state (amount, repaid, defaulted) for a given borrower.
- **Active_Loan**: A LoanRecord that exists in persistent storage for a given borrower, regardless of repaid or defaulted status.
- **Stroops**: The smallest unit of XLM (1 XLM = 10,000,000 stroops).

## Requirements

### Requirement 1: Voucher Authorization

**User Story:** As a voucher, I want only myself to be able to withdraw my stake, so that no other party can remove my funds without my consent.

#### Acceptance Criteria

1. WHEN `withdraw_vouch` is called, THE QuorumCredit_Contract SHALL require authorization from the `voucher` argument before performing any state changes or transfers.
2. IF the `voucher` authorization is not provided, THEN THE QuorumCredit_Contract SHALL reject the transaction with an authorization error.

---

### Requirement 2: Pre-Loan Guard

**User Story:** As a protocol designer, I want withdrawals to be blocked once a loan is active, so that voucher stakes remain locked as collateral during the loan lifecycle.

#### Acceptance Criteria

1. WHEN `withdraw_vouch` is called and a LoanRecord exists for the given `borrower`, THEN THE QuorumCredit_Contract SHALL panic with the message `"loan already active"`.
2. WHEN `withdraw_vouch` is called and no LoanRecord exists for the given `borrower`, THE QuorumCredit_Contract SHALL proceed with the withdrawal.

---

### Requirement 3: Vouch Record Existence

**User Story:** As a voucher, I want the contract to verify my vouch record exists before attempting a withdrawal, so that invalid calls fail with a clear error.

#### Acceptance Criteria

1. WHEN `withdraw_vouch` is called and no VouchRecord exists for the given `voucher`/`borrower` pair, THEN THE QuorumCredit_Contract SHALL panic with the message `"vouch not found"`.
2. WHEN `withdraw_vouch` is called and a matching VouchRecord exists, THE QuorumCredit_Contract SHALL proceed with stake removal and transfer.

---

### Requirement 4: Stake Return

**User Story:** As a voucher, I want my full original stake returned to my account when I withdraw, so that I recover exactly what I deposited.

#### Acceptance Criteria

1. WHEN a valid `withdraw_vouch` call is processed, THE QuorumCredit_Contract SHALL transfer the voucher's exact original stake amount in stroops back to the `voucher` address.
2. THE QuorumCredit_Contract SHALL transfer no more and no less than the recorded stake value (no yield, no penalty).

---

### Requirement 5: Vouch Record Removal

**User Story:** As a protocol designer, I want the vouch record removed from storage after withdrawal, so that the contract state stays consistent and the voucher cannot double-withdraw.

#### Acceptance Criteria

1. WHEN a valid `withdraw_vouch` call is processed, THE QuorumCredit_Contract SHALL remove the VouchRecord for the given `voucher`/`borrower` pair from persistent storage.
2. WHEN the removed VouchRecord was the only entry for the `borrower`, THE QuorumCredit_Contract SHALL remove the entire vouches list for that `borrower` from persistent storage.
3. WHEN other VouchRecords remain for the `borrower` after removal, THE QuorumCredit_Contract SHALL preserve those remaining records unchanged.

---

### Requirement 6: Round-Trip Consistency

**User Story:** As a protocol designer, I want the contract state after a withdraw to be equivalent to a state where the vouch never happened, so that the system remains internally consistent.

#### Acceptance Criteria

1. WHEN a voucher calls `vouch` and then immediately calls `withdraw_vouch` (with no loan requested in between), THE QuorumCredit_Contract SHALL return the voucher's token balance to its pre-vouch value.
2. WHEN a voucher calls `vouch` and then immediately calls `withdraw_vouch`, THE QuorumCredit_Contract SHALL result in `get_vouches` returning a list that does not contain a VouchRecord for that voucher.
