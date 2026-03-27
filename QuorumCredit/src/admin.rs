use crate::helpers::{config, require_admin_approval, validate_admin_config};
use crate::types::{Config, DataKey};
use soroban_sdk::{symbol_short, Address, BytesN, Env, Vec};

pub fn add_admin(env: Env, admin_signers: Vec<Address>, new_admin: Address) {
    require_admin_approval(&env, &admin_signers);

    let mut cfg = config(&env);

    assert!(
        !cfg.admins.iter().any(|a| a == new_admin),
        "address is already an admin"
    );

    cfg.admins.push_back(new_admin.clone());
    env.storage().instance().set(&DataKey::Config, &cfg);

    env.events()
        .publish((symbol_short!("admin"), symbol_short!("added")), new_admin);
}

pub fn remove_admin(env: Env, admin_signers: Vec<Address>, admin_to_remove: Address) {
    require_admin_approval(&env, &admin_signers);

    let mut cfg = config(&env);

    let idx = cfg
        .admins
        .iter()
        .position(|a| a == admin_to_remove)
        .expect("address is not an admin") as u32;

    cfg.admins.remove(idx);

    assert!(!cfg.admins.is_empty(), "cannot remove the last admin");
    assert!(
        cfg.admin_threshold <= cfg.admins.len(),
        "removal would make threshold unsatisfiable"
    );

    env.storage().instance().set(&DataKey::Config, &cfg);

    env.events().publish(
        (symbol_short!("admin"), symbol_short!("removed")),
        admin_to_remove,
    );
}

pub fn rotate_admin(env: Env, admin_signers: Vec<Address>, old_admin: Address, new_admin: Address) {
    require_admin_approval(&env, &admin_signers);

    assert!(old_admin != new_admin, "old and new admin must differ");

    let mut cfg = config(&env);

    assert!(
        !cfg.admins.iter().any(|a| a == new_admin),
        "new admin is already in the admin set"
    );

    let idx = cfg
        .admins
        .iter()
        .position(|a| a == old_admin)
        .expect("old admin not found") as u32;

    cfg.admins.set(idx, new_admin.clone());
    env.storage().instance().set(&DataKey::Config, &cfg);

    env.events().publish(
        (symbol_short!("admin"), symbol_short!("rotated")),
        (old_admin, new_admin),
    );
}

pub fn set_admin_threshold(env: Env, admin_signers: Vec<Address>, new_threshold: u32) {
    require_admin_approval(&env, &admin_signers);

    let mut cfg = config(&env);

    assert!(new_threshold > 0, "threshold must be greater than zero");
    assert!(
        new_threshold <= cfg.admins.len(),
        "threshold cannot exceed admin count"
    );

    cfg.admin_threshold = new_threshold;
    env.storage().instance().set(&DataKey::Config, &cfg);

    env.events().publish(
        (symbol_short!("admin"), symbol_short!("thresh")),
        new_threshold,
    );
}

pub fn set_protocol_fee(env: Env, admin_signers: Vec<Address>, fee_bps: u32) {
    require_admin_approval(&env, &admin_signers);
    assert!(fee_bps <= 10_000, "fee_bps must not exceed 10000");
    env.storage()
        .instance()
        .set(&DataKey::ProtocolFeeBps, &fee_bps);
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("fee")),
        (
            admin_signers.get(0).unwrap(),
            fee_bps,
            env.ledger().timestamp(),
        ),
    );
}

pub fn whitelist_voucher(env: Env, admin_signers: Vec<Address>, voucher: Address) {
    require_admin_approval(&env, &admin_signers);
    env.storage()
        .persistent()
        .set(&DataKey::VoucherWhitelist(voucher), &true);
}

pub fn set_fee_treasury(env: Env, admin_signers: Vec<Address>, treasury: Address) {
    require_admin_approval(&env, &admin_signers);
    env.storage()
        .instance()
        .set(&DataKey::FeeTreasury, &treasury);
}

pub fn upgrade(env: Env, admin_signers: Vec<Address>, new_wasm_hash: BytesN<32>) {
    require_admin_approval(&env, &admin_signers);
    env.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());
    env.events()
        .publish((symbol_short!("upgrade"),), new_wasm_hash);
}

pub fn pause(env: Env, admin_signers: Vec<Address>) {
    require_admin_approval(&env, &admin_signers);
    env.storage().instance().set(&DataKey::Paused, &true);
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("pause")),
        (admin_signers.get(0).unwrap(), env.ledger().timestamp()),
    );
}

pub fn unpause(env: Env, admin_signers: Vec<Address>) {
    require_admin_approval(&env, &admin_signers);
    env.storage().instance().set(&DataKey::Paused, &false);
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("unpause")),
        (admin_signers.get(0).unwrap(), env.ledger().timestamp()),
    );
}

pub fn blacklist(env: Env, admin_signers: Vec<Address>, borrower: Address) {
    require_admin_approval(&env, &admin_signers);
    env.storage()
        .persistent()
        .set(&DataKey::Blacklisted(borrower), &true);
}

pub fn set_config(env: Env, admin_signers: Vec<Address>, config: Config) {
    require_admin_approval(&env, &admin_signers);
    validate_admin_config(&env, &config.admins, config.admin_threshold)
        .expect("invalid admin config");
    assert!(config.yield_bps >= 0, "yield_bps must be non-negative");
    assert!(
        config.slash_bps > 0 && config.slash_bps <= 10_000,
        "slash_bps must be 1-10000"
    );
    assert!(
        config.max_vouchers > 0,
        "max_vouchers must be greater than zero"
    );
    assert!(
        config.min_loan_amount > 0,
        "min_loan_amount must be greater than zero"
    );
    assert!(
        config.loan_duration > 0,
        "loan_duration must be greater than zero"
    );
    assert!(
        config.max_loan_to_stake_ratio > 0,
        "max_loan_to_stake_ratio must be greater than zero"
    );
    env.storage().instance().set(&DataKey::Config, &config);
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("config")),
        (admin_signers.get(0).unwrap(), env.ledger().timestamp()),
    );
}

pub fn update_config(
    env: Env,
    admin_signers: Vec<Address>,
    yield_bps: Option<i128>,
    slash_bps: Option<i128>,
) {
    require_admin_approval(&env, &admin_signers);

    let mut cfg = config(&env);

    if let Some(new_yield_bps) = yield_bps {
        assert!(new_yield_bps >= 0, "yield_bps must be non-negative");
        cfg.yield_bps = new_yield_bps;
    }

    if let Some(new_slash_bps) = slash_bps {
        assert!(
            new_slash_bps > 0 && new_slash_bps <= 10_000,
            "slash_bps must be 1-10000"
        );
        cfg.slash_bps = new_slash_bps;
    }

    env.storage().instance().set(&DataKey::Config, &cfg);
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("upconfig")),
        (admin_signers.get(0).unwrap(), env.ledger().timestamp()),
    );
}

pub fn set_reputation_nft(env: Env, admin_signers: Vec<Address>, nft_contract: Address) {
    require_admin_approval(&env, &admin_signers);
    env.storage()
        .instance()
        .set(&DataKey::ReputationNft, &nft_contract);
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("repnft")),
        (
            admin_signers.get(0).unwrap(),
            nft_contract,
            env.ledger().timestamp(),
        ),
    );
}

pub fn set_min_stake(env: Env, admin_signers: Vec<Address>, amount: i128) {
    require_admin_approval(&env, &admin_signers);
    assert!(amount >= 0, "min stake cannot be negative");
    env.storage().instance().set(&DataKey::MinStake, &amount);
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("minstake")),
        (
            admin_signers.get(0).unwrap(),
            amount,
            env.ledger().timestamp(),
        ),
    );
}

pub fn set_max_loan_amount(env: Env, admin_signers: Vec<Address>, amount: i128) {
    require_admin_approval(&env, &admin_signers);
    assert!(amount >= 0, "max loan amount cannot be negative");
    env.storage()
        .instance()
        .set(&DataKey::MaxLoanAmount, &amount);
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("maxloan")),
        (
            admin_signers.get(0).unwrap(),
            amount,
            env.ledger().timestamp(),
        ),
    );
}

pub fn set_min_vouchers(env: Env, admin_signers: Vec<Address>, count: u32) {
    require_admin_approval(&env, &admin_signers);
    env.storage().instance().set(&DataKey::MinVouchers, &count);
    env.events().publish(
        (symbol_short!("admin"), symbol_short!("minvchrs")),
        (
            admin_signers.get(0).unwrap(),
            count,
            env.ledger().timestamp(),
        ),
    );
}

pub fn set_max_loan_to_stake_ratio(env: Env, admin_signers: Vec<Address>, ratio: u32) {
    require_admin_approval(&env, &admin_signers);
    assert!(
        ratio > 0,
        "max_loan_to_stake_ratio must be greater than zero"
    );
    let mut cfg = config(&env);
    cfg.max_loan_to_stake_ratio = ratio;
    env.storage().instance().set(&DataKey::Config, &cfg);
}

// View functions
pub fn get_protocol_fee(env: Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::ProtocolFeeBps)
        .unwrap_or(0)
}

pub fn get_fee_treasury(env: Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::FeeTreasury)
}

pub fn is_blacklisted(env: Env, borrower: Address) -> bool {
    env.storage()
        .persistent()
        .get::<DataKey, bool>(&DataKey::Blacklisted(borrower))
        .unwrap_or(false)
}

pub fn get_min_stake(env: Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::MinStake)
        .unwrap_or(0)
}

pub fn get_max_loan_amount(env: Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::MaxLoanAmount)
        .unwrap_or(0)
}

pub fn get_min_vouchers(env: Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::MinVouchers)
        .unwrap_or(0)
}

pub fn get_max_loan_to_stake_ratio(env: Env) -> u32 {
    config(&env).max_loan_to_stake_ratio
}

pub fn get_config(env: Env) -> Config {
    config(&env)
}

pub fn add_allowed_token(env: Env, admin_signers: Vec<Address>, token: Address) {
    require_admin_approval(&env, &admin_signers);
    let mut cfg = config(&env);
    assert!(
        !cfg.allowed_tokens.iter().any(|t| t == token) && token != cfg.token,
        "token already allowed"
    );
    cfg.allowed_tokens.push_back(token);
    env.storage().instance().set(&DataKey::Config, &cfg);
}

pub fn remove_allowed_token(env: Env, admin_signers: Vec<Address>, token: Address) {
    require_admin_approval(&env, &admin_signers);
    let mut cfg = config(&env);
    let idx = cfg
        .allowed_tokens
        .iter()
        .position(|t| t == token)
        .expect("token not in allowed list") as u32;
    cfg.allowed_tokens.remove(idx);
    env.storage().instance().set(&DataKey::Config, &cfg);
}

pub fn get_admins(env: Env) -> Vec<Address> {
    config(&env).admins
}

pub fn get_admin_threshold(env: Env) -> u32 {
    config(&env).admin_threshold
}

pub fn is_whitelisted(env: Env, voucher: Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::VoucherWhitelist(voucher))
        .unwrap_or(false)
}
