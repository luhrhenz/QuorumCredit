#[cfg(test)]
mod tests {
    use crate::errors::ContractError;
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Address, Env, String, Vec,
    };

    fn setup(env: &Env) -> (QuorumCreditContractClient, Address, Address) {
        let token_admin = Address::generate(env);
        let token = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(env, &contract_id);
        let deployer = Address::generate(env);
        let admin = Address::generate(env);
        client.initialize(&deployer, &Vec::from_array(env, [admin.clone()]), &1, &token);
        (client, admin, token)
    }

    fn fund_and_vouch(
        env: &Env,
        client: &QuorumCreditContractClient,
        token: &Address,
        contract_id: &Address,
        voucher: &Address,
        borrower: &Address,
    ) {
        let token_client = soroban_sdk::token::StellarAssetClient::new(env, token);
        token_client.mint(voucher, &10_000_000);
        token_client.mint(contract_id, &10_000_000);
        env.ledger().with_mut(|li| li.timestamp = 1000);
        client.vouch(voucher, borrower, &5_000_000, token);
        env.ledger().with_mut(|li| li.timestamp = 1000 + 61);
    }

    #[test]
    fn test_whitelist_disabled_anyone_can_borrow() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);
        fund_and_vouch(&env, &client, &token, &contract_id, &voucher, &borrower);

        // Whitelist is disabled by default — request_loan should succeed
        client.request_loan(
            &borrower,
            &100_000,
            &1_000_000,
            &String::from_str(&env, "test"),
            &token,
        );
    }

    #[test]
    fn test_whitelist_enabled_non_whitelisted_borrower_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);
        fund_and_vouch(&env, &client, &token, &contract_id, &voucher, &borrower);

        client.enable_borrower_whitelist(&Vec::from_array(&env, [admin.clone()]));

        let result = client.try_request_loan(
            &borrower,
            &100_000,
            &1_000_000,
            &String::from_str(&env, "test"),
            &token,
        );
        assert_eq!(result, Err(Ok(ContractError::Blacklisted)));
    }

    #[test]
    fn test_whitelist_enabled_whitelisted_borrower_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);
        fund_and_vouch(&env, &client, &token, &contract_id, &voucher, &borrower);

        let admins = Vec::from_array(&env, [admin.clone()]);
        client.enable_borrower_whitelist(&admins);
        client.add_borrower_to_whitelist(&admins, &borrower);

        client.request_loan(
            &borrower,
            &100_000,
            &1_000_000,
            &String::from_str(&env, "test"),
            &token,
        );
    }

    #[test]
    fn test_remove_borrower_from_whitelist_blocks_loan() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);
        fund_and_vouch(&env, &client, &token, &contract_id, &voucher, &borrower);

        let admins = Vec::from_array(&env, [admin.clone()]);
        client.enable_borrower_whitelist(&admins);
        client.add_borrower_to_whitelist(&admins, &borrower);
        client.remove_borrower_from_whitelist(&admins, &borrower);

        let result = client.try_request_loan(
            &borrower,
            &100_000,
            &1_000_000,
            &String::from_str(&env, "test"),
            &token,
        );
        assert_eq!(result, Err(Ok(ContractError::Blacklisted)));
    }

    #[test]
    fn test_disable_whitelist_allows_all_borrowers() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);
        fund_and_vouch(&env, &client, &token, &contract_id, &voucher, &borrower);

        let admins = Vec::from_array(&env, [admin.clone()]);
        client.enable_borrower_whitelist(&admins);
        client.disable_borrower_whitelist(&admins);

        // After disabling, non-whitelisted borrower should succeed
        client.request_loan(
            &borrower,
            &100_000,
            &1_000_000,
            &String::from_str(&env, "test"),
            &token,
        );
    }
}
