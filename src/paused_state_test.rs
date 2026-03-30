#![cfg(test)]

use crate::{ContractError, QuorumCreditContract, QuorumCreditContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> (Address, token::Client<'a>) {
    let addr = env.register_stellar_asset_contract_v2(admin.clone());
    (addr.clone(), token::Client::new(env, &addr))
}

fn setup_test_env() -> (
    Env,
    QuorumCreditContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, QuorumCreditContract);
    let client = QuorumCreditContractClient::new(&env, &contract_id);

    let deployer = Address::generate(&env);
    let admin = Address::generate(&env);
    let voucher = Address::generate(&env);
    let borrower = Address::generate(&env);

    let (token_addr, token_client) = create_token_contract(&env, &admin);

    let admins = Vec::from_array(&env, [admin.clone()]);
    client.initialize(&deployer, &admins, &1, &token_addr);

    // Mint tokens to voucher and borrower
    token_client.mint(&voucher, &10_000_000);
    token_client.mint(&borrower, &1_000_000);

    (
        env,
        client,
        admin,
        voucher,
        borrower,
        token_addr,
        token_client,
    )
}

#[test]
fn test_vouch_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, _token_client) = setup_test_env();

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Verify contract is paused
    assert_eq!(client.get_paused(), true);

    // Try to vouch - should fail
    let result = client.try_vouch(&voucher, &borrower, &1_000_000, &token_addr);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_batch_vouch_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, _token_client) = setup_test_env();

    let borrowers = Vec::from_array(&env, [borrower.clone()]);
    let stakes = Vec::from_array(&env, [1_000_000i128]);

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to batch vouch - should fail
    let result = client.try_batch_vouch(&voucher, &borrowers, &stakes, &token_addr);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_increase_stake_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, _token_client) = setup_test_env();

    // First vouch while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to increase stake - should fail
    let result = client.try_increase_stake(&voucher, &borrower, &500_000);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_decrease_stake_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, _token_client) = setup_test_env();

    // First vouch while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to decrease stake - should fail
    let result = client.try_decrease_stake(&voucher, &borrower, &500_000);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_withdraw_vouch_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, _token_client) = setup_test_env();

    // First vouch while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to withdraw vouch - should fail
    let result = client.try_withdraw_vouch(&voucher, &borrower);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_transfer_vouch_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, _token_client) = setup_test_env();

    let new_voucher = Address::generate(&env);

    // First vouch while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to transfer vouch - should fail
    let result = client.try_transfer_vouch(&voucher, &new_voucher, &borrower);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_request_loan_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, token_client) = setup_test_env();

    // Setup: vouch while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Fund the contract for loan disbursement
    token_client.mint(
        &env.as_contract(&client.address, || env.current_contract_address()),
        &5_000_000,
    );

    // Advance time to meet MIN_VOUCH_AGE requirement
    env.ledger().with_mut(|li| li.timestamp = 100);

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to request loan - should fail
    let result = client.try_request_loan(
        &borrower,
        &500_000,
        &1_000_000,
        &String::from_str(&env, "test loan"),
        &token_addr,
    );
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_repay_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, token_client) = setup_test_env();

    // Setup: vouch and request loan while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Fund the contract for loan disbursement
    token_client.mint(
        &env.as_contract(&client.address, || env.current_contract_address()),
        &5_000_000,
    );

    // Advance time to meet MIN_VOUCH_AGE requirement
    env.ledger().with_mut(|li| li.timestamp = 100);

    client.request_loan(
        &borrower,
        &500_000,
        &1_000_000,
        &String::from_str(&env, "test loan"),
        &token_addr,
    );

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to repay - should fail
    let result = client.try_repay(&borrower, &100_000);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_vote_slash_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, token_client) = setup_test_env();

    // Setup: vouch and request loan while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Fund the contract for loan disbursement
    token_client.mint(
        &env.as_contract(&client.address, || env.current_contract_address()),
        &5_000_000,
    );

    // Advance time to meet MIN_VOUCH_AGE requirement
    env.ledger().with_mut(|li| li.timestamp = 100);

    client.request_loan(
        &borrower,
        &500_000,
        &1_000_000,
        &String::from_str(&env, "test loan"),
        &token_addr,
    );

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to vote slash - should fail
    let result = client.try_vote_slash(&voucher, &borrower, &true);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_propose_slash_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, token_client) = setup_test_env();

    // Setup: vouch and request loan while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Fund the contract for loan disbursement
    token_client.mint(
        &env.as_contract(&client.address, || env.current_contract_address()),
        &5_000_000,
    );

    // Advance time to meet MIN_VOUCH_AGE requirement
    env.ledger().with_mut(|li| li.timestamp = 100);

    client.request_loan(
        &borrower,
        &500_000,
        &1_000_000,
        &String::from_str(&env, "test loan"),
        &token_addr,
    );

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to propose slash - should fail
    let result = client.try_propose_slash(&admin, &borrower, &86400);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_execute_slash_proposal_blocked_when_paused() {
    let (env, client, admin, voucher, borrower, token_addr, token_client) = setup_test_env();

    // Setup: vouch and request loan while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Fund the contract for loan disbursement
    token_client.mint(
        &env.as_contract(&client.address, || env.current_contract_address()),
        &5_000_000,
    );

    // Advance time to meet MIN_VOUCH_AGE requirement
    env.ledger().with_mut(|li| li.timestamp = 100);

    client.request_loan(
        &borrower,
        &500_000,
        &1_000_000,
        &String::from_str(&env, "test loan"),
        &token_addr,
    );

    // Propose slash while unpaused
    let proposal_id = client.propose_slash(&admin, &borrower, &86400);

    // Advance time past the delay
    env.ledger().with_mut(|li| li.timestamp = 86600);

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to execute slash proposal - should fail
    let result = client.try_execute_slash_proposal(&proposal_id);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_register_referral_blocked_when_paused() {
    let (env, client, admin, _voucher, borrower, _token_addr, _token_client) = setup_test_env();

    let referrer = Address::generate(&env);

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Try to register referral - should fail
    let result = client.try_register_referral(&borrower, &referrer);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_operations_work_after_unpause() {
    let (env, client, admin, voucher, borrower, token_addr, token_client) = setup_test_env();

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Verify contract is paused
    assert_eq!(client.get_paused(), true);

    // Try to vouch - should fail
    let result = client.try_vouch(&voucher, &borrower, &1_000_000, &token_addr);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));

    // Unpause the contract
    client.unpause(&admin_signers);

    // Verify contract is unpaused
    assert_eq!(client.get_paused(), false);

    // Now vouch should work
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Verify vouch was successful
    assert!(client.vouch_exists(&voucher, &borrower));
}

#[test]
fn test_all_fund_moving_functions_respect_pause() {
    let (env, client, admin, voucher, borrower, token_addr, token_client) = setup_test_env();

    let referrer = Address::generate(&env);
    let new_voucher = Address::generate(&env);

    // Setup: create vouches and loan while unpaused
    client.vouch(&voucher, &borrower, &1_000_000, &token_addr);

    // Fund the contract for loan disbursement
    token_client.mint(
        &env.as_contract(&client.address, || env.current_contract_address()),
        &5_000_000,
    );

    // Advance time to meet MIN_VOUCH_AGE requirement
    env.ledger().with_mut(|li| li.timestamp = 100);

    client.request_loan(
        &borrower,
        &500_000,
        &1_000_000,
        &String::from_str(&env, "test loan"),
        &token_addr,
    );

    // Pause the contract
    let admin_signers = Vec::from_array(&env, [admin.clone()]);
    client.pause(&admin_signers);

    // Test all fund-moving functions are blocked
    assert_eq!(
        client.try_vouch(&voucher, &Address::generate(&env), &1_000_000, &token_addr),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_eq!(
        client.try_increase_stake(&voucher, &borrower, &500_000),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_eq!(
        client.try_decrease_stake(&voucher, &borrower, &100_000),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_eq!(
        client.try_withdraw_vouch(&voucher, &borrower),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_eq!(
        client.try_transfer_vouch(&voucher, &new_voucher, &borrower),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_eq!(
        client.try_repay(&borrower, &100_000),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_eq!(
        client.try_vote_slash(&voucher, &borrower, &true),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_eq!(
        client.try_register_referral(&Address::generate(&env), &referrer),
        Err(Ok(ContractError::ContractPaused))
    );
}
