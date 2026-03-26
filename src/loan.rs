use crate::errors::ContractError;
use crate::helpers::{
    config, get_active_loan_record, has_active_loan, next_loan_id, require_not_paused, token,
};
use crate::reputation::ReputationNftExternalClient;
use crate::types::{DataKey, LoanRecord, LoanStatus, VouchRecord, MIN_VOUCH_AGE};
use soroban_sdk::{symbol_short, Address, Env, Vec};

pub fn request_loan(
    env: Env,
    borrower: Address,
    amount: i128,
    threshold: i128,
) -> Result<(), ContractError> {
    borrower.require_auth();
    require_not_paused(&env)?;

    if env
        .storage()
        .persistent()
        .get::<DataKey, bool>(&DataKey::Blacklisted(borrower.clone()))
        .unwrap_or(false)
    {
        return Err(ContractError::Blacklisted);
    }

    let cfg = config(&env);

    assert!(
        amount >= cfg.min_loan_amount,
        "loan amount must meet minimum threshold"
    );
    // Validate threshold is strictly positive.
    assert!(threshold > 0, "threshold must be greater than zero");

    // Enforce max loan amount cap if configured.
    let max_loan_amount: i128 = env
        .storage()
        .instance()
        .get(&DataKey::MaxLoanAmount)
        .unwrap_or(0);
    if max_loan_amount > 0 && amount > max_loan_amount {
        return Err(ContractError::LoanExceedsMaxAmount);
    }

    // Prevent overwriting an active loan record.
    assert!(
        !has_active_loan(&env, &borrower),
        "borrower already has an active loan"
    );

    let vouches: Vec<VouchRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::Vouches(borrower.clone()))
        .unwrap_or(Vec::new(&env));

    let mut total_stake: i128 = 0;
    for v in vouches.iter() {
        total_stake = total_stake
            .checked_add(v.stake)
            .ok_or(ContractError::StakeOverflow)?;
    }
    assert!(total_stake >= threshold, "insufficient trust stake");

    // Enforce minimum voucher count if configured.
    let min_vouchers: u32 = env
        .storage()
        .instance()
        .get(&DataKey::MinVouchers)
        .unwrap_or(0);
    if vouches.len() < min_vouchers {
        return Err(ContractError::InsufficientVouchers);
    }

    // Enforce minimum vouch age: every vouch must be at least MIN_VOUCH_AGE seconds old.
    // This prevents a same-transaction (or same-block) vouch → request_loan attack.
    let now = env.ledger().timestamp();
    for v in vouches.iter() {
        if now < v.vouch_timestamp + MIN_VOUCH_AGE {
            return Err(ContractError::VouchTooRecent);
        }
    }

    // Check collateral ratio: amount must not exceed total_stake * ratio / 100
    let max_allowed_loan = total_stake * cfg.max_loan_to_stake_ratio as i128 / 100;
    assert!(
        amount <= max_allowed_loan,
        "loan amount exceeds maximum collateral ratio"
    );

    // Verify the contract holds enough XLM to cover the loan.
    let token = token(&env);
    let contract_balance = token.balance(&env.current_contract_address());
    if contract_balance < amount {
        return Err(ContractError::InsufficientFunds);
    }

    let deadline = now + cfg.loan_duration;
    let loan_id = next_loan_id(&env);

    // Lock in the yield at disbursement time so rate changes mid-loan don't
    // affect what the borrower owes or what vouchers receive (fixes issue #15).
    let total_yield = amount * cfg.yield_bps / 10_000;

    env.storage().persistent().set(
        &DataKey::Loan(loan_id),
        &LoanRecord {
            id: loan_id,
            borrower: borrower.clone(),
            co_borrowers: Vec::new(&env),
            amount,
            amount_repaid: 0,
            total_yield,
            repaid: false,
            defaulted: false,
            created_at: now,
            disbursement_timestamp: now,
            deadline,
        },
    );
    env.storage()
        .persistent()
        .set(&DataKey::ActiveLoan(borrower.clone()), &loan_id);
    env.storage()
        .persistent()
        .set(&DataKey::LatestLoan(borrower.clone()), &loan_id);

    // Track total historical loan count for this borrower.
    let count: u32 = env
        .storage()
        .persistent()
        .get(&DataKey::LoanCount(borrower.clone()))
        .unwrap_or(0);
    env.storage()
        .persistent()
        .set(&DataKey::LoanCount(borrower.clone()), &(count + 1));

    token.transfer(&env.current_contract_address(), &borrower, &amount);

    env.events().publish(
        (symbol_short!("loan"), symbol_short!("disbursed")),
        (borrower.clone(), amount, deadline),
    );

    Ok(())
}

pub fn repay(env: Env, borrower: Address, payment: i128) -> Result<(), ContractError> {
    borrower.require_auth();
    require_not_paused(&env)?;

    let mut loan = get_active_loan_record(&env, &borrower)?;

    for cb in loan.co_borrowers.iter() {
        cb.require_auth();
    }

    if borrower != loan.borrower {
        return Err(ContractError::UnauthorizedCaller);
    }
    if loan.defaulted || loan.repaid {
        return Err(ContractError::NoActiveLoan);
    }

    assert!(!loan.defaulted, "loan already defaulted");
    assert!(!loan.repaid, "loan already repaid");
    assert!(
        env.ledger().timestamp() <= loan.deadline,
        "loan deadline has passed"
    );

    // Total obligation = principal + yield locked in at disbursement.
    let total_owed = loan.amount + loan.total_yield;
    let outstanding = total_owed - loan.amount_repaid;
    assert!(
        payment > 0 && payment <= outstanding,
        "invalid payment amount"
    );

    let token = token(&env);

    token.transfer(&borrower, &env.current_contract_address(), &payment);
    loan.amount_repaid += payment;
    let fully_repaid = loan.amount_repaid >= total_owed;

    if fully_repaid {
        let vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower.clone()))
            .unwrap_or(Vec::new(&env));
        let total_stake: i128 = vouches.iter().map(|v| v.stake).sum();

        for v in vouches.iter() {
            let voucher_yield = if total_stake > 0 {
                loan.total_yield * v.stake / total_stake
            } else {
                0
            };
            token.transfer(
                &env.current_contract_address(),
                &v.voucher,
                &(v.stake + voucher_yield),
            );
        }

        loan.repaid = true;
        let count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::RepaymentCount(borrower.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::RepaymentCount(borrower.clone()), &(count + 1));

        if let Some(nft_addr) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::ReputationNft)
        {
            ReputationNftExternalClient::new(&env, &nft_addr).mint(&borrower);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::ActiveLoan(borrower.clone()));
        env.storage()
            .persistent()
            .remove(&DataKey::Vouches(borrower.clone()));

        env.events().publish(
            (symbol_short!("loan"), symbol_short!("repaid")),
            (borrower.clone(), loan.amount),
        );
    }

    env.storage()
        .persistent()
        .set(&DataKey::Loan(loan.id), &loan);

    Ok(())
}

pub fn loan_status(env: Env, borrower: Address) -> LoanStatus {
    match crate::helpers::get_latest_loan_record(&env, &borrower) {
        None => LoanStatus::None,
        Some(loan) if loan.repaid => LoanStatus::Repaid,
        Some(loan) if loan.defaulted => LoanStatus::Defaulted,
        _ => LoanStatus::Active,
    }
}

pub fn get_loan(env: Env, borrower: Address) -> Option<LoanRecord> {
    crate::helpers::get_latest_loan_record(&env, &borrower)
}

pub fn get_loan_by_id(env: Env, loan_id: u64) -> Option<LoanRecord> {
    env.storage().persistent().get(&DataKey::Loan(loan_id))
}

pub fn is_eligible(env: Env, borrower: Address, threshold: i128) -> bool {
    if threshold <= 0 {
        return false;
    }

    if let Some(loan) = crate::helpers::get_latest_loan_record(&env, &borrower) {
        if !loan.repaid && !loan.defaulted {
            return false;
        }
    }

    let vouches: Vec<VouchRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::Vouches(borrower))
        .unwrap_or(Vec::new(&env));

    let total_stake: i128 = vouches.iter().map(|v| v.stake).sum();
    total_stake >= threshold
}

pub fn repayment_count(env: Env, borrower: Address) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::RepaymentCount(borrower))
        .unwrap_or(0)
}

pub fn loan_count(env: Env, borrower: Address) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::LoanCount(borrower))
        .unwrap_or(0)
}

pub fn default_count(env: Env, borrower: Address) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::DefaultCount(borrower))
        .unwrap_or(0)
}
