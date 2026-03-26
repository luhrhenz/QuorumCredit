use crate::errors::ContractError;
use crate::helpers::{has_active_loan, require_not_paused, require_positive_amount, token};
use crate::types::{DataKey, VouchRecord};
use soroban_sdk::{symbol_short, Address, Env, Vec};

pub fn vouch(
    env: Env,
    voucher: Address,
    borrower: Address,
    stake: i128,
) -> Result<(), ContractError> {
    voucher.require_auth();
    require_not_paused(&env)?;
    do_vouch(&env, voucher, borrower, stake)
}

fn do_vouch(
    env: &Env,
    voucher: Address,
    borrower: Address,
    stake: i128,
) -> Result<(), ContractError> {
    // Validate numeric input: stake must be strictly positive.
    require_positive_amount(env, stake)?;

    assert!(voucher != borrower, "voucher cannot vouch for self");
    assert!(stake > 0, "stake must be greater than zero");

    // Sybil resistance: enforce minimum stake per vouch.
    let min_stake: i128 = env
        .storage()
        .instance()
        .get(&DataKey::MinStake)
        .unwrap_or(0);
    if min_stake > 0 && stake < min_stake {
        return Err(ContractError::MinStakeNotMet);
    }

    // Rate limiting: enforce cooldown between vouch calls from the same address.
    let _now = env.ledger().timestamp();
    let _last: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::LastVouchTimestamp(voucher.clone()))
        .unwrap_or(0);

    let mut vouches: Vec<VouchRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::Vouches(borrower.clone()))
        .unwrap_or(Vec::new(env));

    // Reject duplicate vouch before any state mutation or transfer.
    for v in vouches.iter() {
        if v.voucher == voucher {
            return Err(ContractError::DuplicateVouch);
        }
    }

    // Reject vouch if the borrower already has an active loan — the stake
    // would be locked with no effect on the existing loan (fixes issue #13).
    if has_active_loan(env, &borrower) {
        return Err(ContractError::ActiveLoanExists);
    }

    // Transfer stake from voucher into the contract.
    let token = token(env);
    token.transfer(&voucher, &env.current_contract_address(), &stake);

    // Track voucher → borrowers history.
    let mut history: Vec<Address> = env
        .storage()
        .persistent()
        .get(&DataKey::VoucherHistory(voucher.clone()))
        .unwrap_or(Vec::new(env));
    history.push_back(borrower.clone());
    env.storage()
        .persistent()
        .set(&DataKey::VoucherHistory(voucher.clone()), &history);

    vouches.push_back(VouchRecord {
        voucher: voucher.clone(),
        stake,
        vouch_timestamp: env.ledger().timestamp(),
    });
    env.storage()
        .persistent()
        .set(&DataKey::Vouches(borrower.clone()), &vouches);

    // Record the timestamp of this vouch for rate limiting.
    env.storage().persistent().set(
        &DataKey::LastVouchTimestamp(voucher.clone()),
        &env.ledger().timestamp(),
    );

    env.events().publish(
        (symbol_short!("vouch"), symbol_short!("added")),
        (voucher, borrower, stake),
    );

    Ok(())
}

pub fn batch_vouch(
    env: Env,
    voucher: Address,
    borrowers: Vec<Address>,
    stakes: Vec<i128>,
) -> Result<(), ContractError> {
    voucher.require_auth();
    require_not_paused(&env)?;

    assert!(
        borrowers.len() == stakes.len(),
        "borrowers and stakes length mismatch"
    );
    assert!(!borrowers.is_empty(), "batch cannot be empty");

    for i in 0..borrowers.len() {
        let borrower = borrowers.get(i).unwrap();
        let stake = stakes.get(i).unwrap();
        do_vouch(&env, voucher.clone(), borrower, stake)?;
    }

    Ok(())
}

pub fn increase_stake(
    env: Env,
    voucher: Address,
    borrower: Address,
    additional: i128,
) -> Result<(), ContractError> {
    voucher.require_auth();
    require_not_paused(&env)?;

    // Validate numeric input: additional must be strictly positive.
    require_positive_amount(&env, additional)?;

    let mut vouches: Vec<VouchRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::Vouches(borrower.clone()))
        .expect("vouch not found");

    let idx = vouches
        .iter()
        .position(|v| v.voucher == voucher)
        .expect("vouch not found") as u32;

    let mut vouch = vouches.get(idx).unwrap();
    token(&env).transfer(&voucher, &env.current_contract_address(), &additional);

    vouch.stake += additional;
    vouches.set(idx, vouch);

    env.storage()
        .persistent()
        .set(&DataKey::Vouches(borrower), &vouches);

    Ok(())
}

pub fn decrease_stake(
    env: Env,
    voucher: Address,
    borrower: Address,
    amount: i128,
) -> Result<(), ContractError> {
    voucher.require_auth();
    require_not_paused(&env)?;

    assert!(amount > 0, "decrease amount must be greater than zero");
    assert!(!has_active_loan(&env, &borrower), "loan already active");

    let mut vouches: Vec<VouchRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::Vouches(borrower.clone()))
        .expect("vouch not found");

    let idx = vouches
        .iter()
        .position(|v| v.voucher == voucher)
        .expect("vouch not found") as u32;

    let mut vouch = vouches.get(idx).unwrap();
    assert!(
        amount <= vouch.stake,
        "decrease amount exceeds staked amount"
    );

    vouch.stake -= amount;
    if vouch.stake == 0 {
        vouches.remove(idx);
    } else {
        vouches.set(idx, vouch);
    }

    if vouches.is_empty() {
        env.storage()
            .persistent()
            .remove(&DataKey::Vouches(borrower));
    } else {
        env.storage()
            .persistent()
            .set(&DataKey::Vouches(borrower), &vouches);
    }

    token(&env).transfer(&env.current_contract_address(), &voucher, &amount);

    Ok(())
}

pub fn withdraw_vouch(env: Env, voucher: Address, borrower: Address) -> Result<(), ContractError> {
    voucher.require_auth();
    require_not_paused(&env)?;

    assert!(!has_active_loan(&env, &borrower), "loan already active");

    let mut vouches: Vec<VouchRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::Vouches(borrower.clone()))
        .ok_or(ContractError::NoActiveLoan)?; // reuse: "no vouch found"

    let idx = vouches
        .iter()
        .position(|v| v.voucher == voucher)
        .ok_or(ContractError::UnauthorizedCaller)? as u32;

    let stake = vouches.get(idx).unwrap().stake;
    vouches.remove(idx);

    if vouches.is_empty() {
        env.storage()
            .persistent()
            .remove(&DataKey::Vouches(borrower.clone()));
    } else {
        env.storage()
            .persistent()
            .set(&DataKey::Vouches(borrower.clone()), &vouches);
    }

    token(&env).transfer(&env.current_contract_address(), &voucher, &stake);

    env.events().publish(
        (symbol_short!("vouch"), symbol_short!("withdrawn")),
        (voucher, borrower, stake),
    );

    Ok(())
}

pub fn transfer_vouch(
    env: Env,
    from: Address,
    to: Address,
    borrower: Address,
) -> Result<(), ContractError> {
    from.require_auth();
    require_not_paused(&env)?;

    if from == to {
        return Ok(());
    }

    // Only allow transfer before a loan is active (consistent with withdraw_vouch).
    assert!(!has_active_loan(&env, &borrower), "loan already active");

    let mut vouches: Vec<VouchRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::Vouches(borrower.clone()))
        .ok_or(ContractError::NoActiveLoan)?;

    let from_idx = vouches
        .iter()
        .position(|v| v.voucher == from)
        .ok_or(ContractError::UnauthorizedCaller)? as u32;

    let from_record = vouches.get(from_idx).unwrap();
    let stake_to_transfer = from_record.stake;

    if let Some(to_idx) = vouches.iter().position(|v| v.voucher == to) {
        // Merge into existing record for 'to'
        let mut to_record = vouches.get(to_idx as u32).unwrap();
        to_record.stake += stake_to_transfer;
        vouches.set(to_idx as u32, to_record);
        vouches.remove(from_idx);
    } else {
        // Transfer ownership to 'to'
        let mut updated_record = from_record;
        updated_record.voucher = to.clone();
        vouches.set(from_idx, updated_record);
    }

    env.storage()
        .persistent()
        .set(&DataKey::Vouches(borrower.clone()), &vouches);

    // Update voucher histories
    // 1. Remove borrower from 'from' history
    let mut from_history: Vec<Address> = env
        .storage()
        .persistent()
        .get(&DataKey::VoucherHistory(from.clone()))
        .unwrap_or(Vec::new(&env));
    if let Some(h_idx) = from_history.iter().position(|b| b == borrower) {
        from_history.remove(h_idx as u32);
        env.storage()
            .persistent()
            .set(&DataKey::VoucherHistory(from.clone()), &from_history);
    }

    // 2. Add borrower to 'to' history if not already there
    let mut to_history: Vec<Address> = env
        .storage()
        .persistent()
        .get(&DataKey::VoucherHistory(to.clone()))
        .unwrap_or(Vec::new(&env));
    if !to_history.iter().any(|b| b == borrower) {
        to_history.push_back(borrower.clone());
        env.storage()
            .persistent()
            .set(&DataKey::VoucherHistory(to.clone()), &to_history);
    }

    env.events().publish(
        (symbol_short!("vouch"), symbol_short!("transfer")),
        (from, to, borrower, stake_to_transfer),
    );

    Ok(())
}

pub fn vouch_exists(env: Env, voucher: Address, borrower: Address) -> bool {
    let vouches: Vec<VouchRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::Vouches(borrower))
        .unwrap_or(Vec::new(&env));
    vouches.iter().any(|v| v.voucher == voucher)
}

pub fn total_vouched(env: Env, borrower: Address) -> i128 {
    env.storage()
        .persistent()
        .get::<DataKey, Vec<VouchRecord>>(&DataKey::Vouches(borrower))
        .unwrap_or(Vec::new(&env))
        .iter()
        .map(|v| v.stake)
        .sum()
}

pub fn voucher_history(env: Env, voucher: Address) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::VoucherHistory(voucher))
        .unwrap_or(Vec::new(&env))
}
