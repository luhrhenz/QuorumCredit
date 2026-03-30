#![allow(unused)]

//! # Stroop Unit Convention
//!
//! **All monetary amounts in this contract are denominated in stroops.**
//!
//! | Unit  | Value                      |
//! |-------|----------------------------|
//! | 1 XLM | 10,000,000 stroops         |
//! | 1 stroop | 0.0000001 XLM           |
//!
//! This applies to every `i128` field or parameter that represents a token
//! amount (stakes, loan principals, yield, fees, minimums, etc.).
//! When displaying values to end-users, divide by `10_000_000` to convert
//! to XLM. When accepting user input in XLM, multiply by `10_000_000`
//! before passing to contract functions.

use soroban_sdk::{contracttype, Address, Vec};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Yield earned by vouchers on full repayment, in basis points (200 = 2%).
pub const DEFAULT_YIELD_BPS: i128 = 200;
/// Fraction of stake burned when a borrower defaults, in basis points (5000 = 50%).
pub const DEFAULT_SLASH_BPS: i128 = 5000;
/// Minimum stake amount, in stroops (50 stroops), required for non-zero yield at
/// the default 2% rate. Amounts below this truncate to zero yield.
/// 1 XLM = 10,000,000 stroops.
pub const DEFAULT_MIN_YIELD_STAKE: i128 = 50;
/// Referral bonus paid to the referrer on full repayment, in basis points (100 = 1% of loan amount).
pub const DEFAULT_REFERRAL_BONUS_BPS: u32 = 100; // 1% of loan amount
/// Minimum age of a vouch before it can be used for a loan, in seconds (60 = 1 minute).
pub const MIN_VOUCH_AGE: u64 = 60; // 1 minute
/// Default maximum number of distinct vouchers per borrower.
pub const DEFAULT_MAX_VOUCHERS: u32 = 100;
/// Default minimum loan amount, in stroops (100,000 stroops = 0.01 XLM).
/// 1 XLM = 10,000,000 stroops.
pub const DEFAULT_MIN_LOAN_AMOUNT: i128 = 100_000;
/// Default loan duration, in seconds (30 days).
pub const DEFAULT_LOAN_DURATION: u64 = 30 * 24 * 60 * 60;
/// Default maximum loan-to-stake ratio (150 = 150% — loan ≤ 1.5× total staked).
pub const DEFAULT_MAX_LOAN_TO_STAKE_RATIO: u32 = 150;
/// Minimum elapsed time between vouch calls from the same address, in seconds (24 hours).
pub const DEFAULT_VOUCH_COOLDOWN_SECS: u64 = 24 * 60 * 60; // 24 hours
/// Default maximum number of vouchers that may back a single borrower.
pub const DEFAULT_MAX_VOUCHERS_PER_BORROWER: u32 = 50;
/// Minimum delay before a timelocked governance action may be executed, in seconds (24 hours).
pub const TIMELOCK_DELAY: u64 = 24 * 60 * 60;
/// Maximum window after `eta` within which a timelocked action must be executed, in seconds (72 hours).
pub const TIMELOCK_EXPIRY: u64 = 72 * 60 * 60;

// ── Loan Status ───────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoanStatus {
    None,
    Active,
    Repaid,
    Defaulted,
}

// ── Storage Keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Loan(u64),                   // loan_id → LoanRecord
    ActiveLoan(Address),         // borrower → active loan_id
    LatestLoan(Address),         // borrower → latest loan_id
    Vouches(Address),            // borrower → Vec<VouchRecord>
    VoucherHistory(Address),     // voucher → Vec<Address> (borrowers backed)
    Config,                      // Config struct: all configurable protocol parameters
    Deployer,                    // Address that deployed the contract; guards initialize
    SlashTreasury,               // i128 accumulated slashed funds
    Paused,                      // bool: true when contract is paused
    BorrowerList,                // Vec<Address> of all borrowers who have ever requested a loan
    ReputationNft,               // Address of the ReputationNftContract
    MinStake,                    // i128 minimum stake amount per vouch
    MaxLoanAmount,               // i128 maximum individual loan size (0 = no cap)
    MinVouchers,     // u32 minimum number of distinct vouchers required (0 = no minimum)
    LoanCounter,     // u64: monotonically increasing loan ID counter
    LoanPool(u64),   // pool_id → LoanPoolRecord
    LoanPoolCounter, // u64: monotonically increasing pool ID counter
    PendingAdmin,    // Address of the pending admin (two-step transfer)
    RepaymentCount(Address), // borrower → u32 total successful repayments
    LoanCount(Address), // borrower → u32 total historical loans disbursed
    DefaultCount(Address), // borrower → u32 total defaults (slash + auto_slash + claim_expired)
    ProtocolFeeBps,  // u32: protocol fee in basis points
    FeeTreasury,     // Address: recipient of collected protocol fees
    LastVouchTimestamp(Address), // voucher → u64 last vouch timestamp
    VouchCooldownSecs, // u64 cooldown between vouch calls (default 24 hours)
    Timelock(u64),   // proposal_id → TimelockProposal
    TimelockCounter, // u64 monotonically increasing proposal ID
    Blacklisted(Address), // borrower → bool permanently banned
    VoucherWhitelist(Address), // voucher → bool allowed to vouch
    WhitelistEnabled, // bool: true when voucher whitelist is enabled (opt-in)
    ExtensionConsents(Address), // borrower → Vec<Address> vouchers who consented to extension
    SlashVote(Address), // borrower → SlashVoteRecord
    SlashVoteQuorum, // u32 quorum in basis points (e.g. 5000 = 50%)
    ReferredBy(Address), // borrower → Address of referrer
    ReferralBonusBps, // u32 referral bonus in basis points (default 100 = 1%)
    MaxVouchersPerBorrower, // u32 maximum number of vouchers per borrower (default 50)
}

// ── Governance ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct SlashVoteRecord {
    /// Total stake (in stroops) that has voted to approve this slash.
    /// 1 XLM = 10,000,000 stroops.
    pub approve_stake: i128,
    /// Total stake (in stroops) that has voted to reject this slash.
    /// 1 XLM = 10,000,000 stroops.
    pub reject_stake: i128,
    /// Addresses that have already cast a vote on this proposal.
    pub voters: Vec<Address>,
    /// `true` once the slash has been auto-executed after quorum was reached.
    pub executed: bool,
}

// ── Config ────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct Config {
    pub admins: Vec<Address>,
    pub admin_threshold: u32,
    /// Primary token contract address used for loans and vouches.
    pub token: Address,
    /// Additional token contract addresses accepted for loans/vouches.
    pub allowed_tokens: Vec<Address>,
    /// Yield rate in basis points (e.g. 200 = 2%). Applied to loan principal (in stroops)
    /// at repayment: `yield = principal_stroops * yield_bps / 10_000`.
    pub yield_bps: i128,
    /// Slash fraction in basis points (e.g. 5000 = 50%). Applied to voucher stake (in stroops)
    /// on borrower default: `slashed = stake_stroops * slash_bps / 10_000`.
    pub slash_bps: i128,
    pub max_vouchers: u32,
    /// Minimum loan amount, in stroops. 1 XLM = 10,000,000 stroops.
    pub min_loan_amount: i128,
    /// Maximum loan duration, in seconds.
    pub loan_duration: u64,
    /// Maximum ratio of loan amount to total staked collateral, expressed as a percentage
    /// (e.g. 150 means loan ≤ 1.5 × total stake in stroops).
    pub max_loan_to_stake_ratio: u32,
    /// Grace period after loan deadline before the loan can be slashed, in seconds.
    pub grace_period: u64,
}

// ── Data Types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct LoanRecord {
    pub id: u64,
    pub borrower: Address,
    pub co_borrowers: Vec<Address>,
    /// Total loan principal disbursed, in stroops. 1 XLM = 10,000,000 stroops.
    pub amount: i128,
    /// Cumulative repayments received so far (principal + yield), in stroops.
    /// 1 XLM = 10,000,000 stroops.
    pub amount_repaid: i128,
    /// Yield owed to vouchers, locked in at disbursement time, in stroops.
    /// Computed as `amount * yield_bps / 10_000`. 1 XLM = 10,000,000 stroops.
    pub total_yield: i128,
    pub status: LoanStatus,
    /// Ledger timestamp when the loan record was created.
    pub created_at: u64,
    /// Ledger timestamp when the loan was disbursed to the borrower.
    pub disbursement_timestamp: u64,
    /// Ledger timestamp when the loan was fully repaid; `None` if not yet repaid.
    pub repayment_timestamp: Option<u64>,
    /// Repayment deadline as a ledger timestamp.
    pub deadline: u64,
    /// Borrower-supplied description of the loan purpose.
    pub loan_purpose: soroban_sdk::String,
    /// Address of the token contract used for this loan.
    pub token_address: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct VouchRecord {
    pub voucher: Address,
    /// Amount staked by the voucher, in stroops. 1 XLM = 10,000,000 stroops.
    pub stake: i128,
    /// Ledger timestamp when this vouch was created; immutable after set.
    pub vouch_timestamp: u64,
    /// Token contract address that this stake is denominated in.
    pub token: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct LoanPoolRecord {
    pub pool_id: u64,
    pub borrowers: Vec<Address>,
    /// Per-borrower loan amounts in this pool, in stroops. 1 XLM = 10,000,000 stroops.
    pub amounts: Vec<i128>,
    /// Ledger timestamp when this pool was created.
    pub created_at: u64,
    /// Total amount disbursed from this pool across all borrowers, in stroops.
    /// 1 XLM = 10,000,000 stroops.
    pub total_disbursed: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct TimelockProposal {
    pub id: u64,
    pub action: TimelockAction,
    pub proposer: Address,
    pub eta: u64,
    pub executed: bool,
    pub cancelled: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum TimelockAction {
    Slash(Address),
    SetConfig(Config),
}
