# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [0.1.0] - 2026-03-30

### Added

- Core vouching system: `vouch()`, `increase_stake()`, `decrease_stake()`, `withdraw_vouch()`, `transfer_vouch()`, `batch_vouch()`
- Loan lifecycle: `request_loan()`, `repay()`, `slash()` with full XLM token transfers via SEP-41
- 2% yield distribution to vouchers on successful repayment (`yield_bps` configurable)
- 50% stake slash on borrower default (`slash_bps` configurable)
- Deployer-gated `initialize()` — prevents front-running between deploy and init
- Multisig admin system: configurable `admin_threshold` requiring M-of-N signatures for all admin operations
- Governance: `initiate_slash_vote()`, `cast_slash_vote()`, `execute_slash_vote()` for decentralised default resolution
- Timelock system for governance operations with configurable delay and expiry window
- Contract pause/unpause (`pause()`, `unpause()`) for emergency halts
- WASM upgrade path via `upgrade()` — requires admin quorum
- Borrower blacklisting (`blacklist()`, `is_blacklisted()`)
- Multi-asset support: primary token + admin-approved `allowed_tokens` list
- `InvalidToken` guard: rejects addresses that do not implement the SEP-41 interface
- Minimum stake enforcement (`set_min_stake()`, `MinStakeNotMet` error)
- Minimum voucher count enforcement (`set_min_vouchers()`, `InsufficientVouchers` error)
- Maximum loan amount cap (`set_max_loan_amount()`, `LoanExceedsMaxAmount` error)
- Maximum loan-to-stake ratio (`set_max_loan_to_stake_ratio()`)
- Vouch age requirement (`MIN_VOUCH_AGE`) to prevent last-minute sybil vouches
- Referral system: `register_referral()` with configurable bonus BPS
- Loan purpose field on `request_loan()` for off-chain auditability
- Reputation NFT integration: mints on successful repayment via external contract
- Persistent TTL extension on all storage writes (~1 year retention)
- `loan_status()`, `get_loan()`, `get_loan_by_id()`, `repayment_count()`, `loan_count()`, `default_count()` view functions
- `vouch_exists()`, `total_vouched()`, `voucher_history()` view functions
- `get_config()`, `get_admins()`, `get_admin_threshold()`, `get_min_stake()`, `get_max_loan_amount()`, `get_min_vouchers()` view functions
- Full error reference: 34 typed `ContractError` variants documented in README
- CI: `cargo audit` for dependency vulnerability scanning; WASM release workflow
- Testnet deployment scripts and invoke scripts for `repay` and `slash`

### Security

- `initialize()` requires deployer signature — closes front-running window between deploy and init (#202 guard)
- Contract balance checked before loan disbursement — prevents over-disbursement
- Yield distribution scoped to loan token only — prevents cross-token fund leakage (issue #112)
- Slash treasury balance isolated from yield payouts
- Duplicate vouch rejected before any state mutation or token transfer
- `AlreadyRepaid` guard prevents double-repayment

### Breaking Changes

- None — initial release.

---

[Unreleased]: https://github.com/your-org/QuorumCredit/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/your-org/QuorumCredit/releases/tag/v0.1.0
