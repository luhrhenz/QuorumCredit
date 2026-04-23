#[cfg(test)]
mod tests {
    use crate::errors::ContractError;
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

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

    #[test]
    fn test_whitelist_disabled_anyone_can_vouch() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);

        let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        token_client.mint(&voucher, &1_000_000);
        token_client.mint(&contract_id, &1_000_000);

        // Whitelist disabled by default — vouch should succeed
        client.vouch(&voucher, &borrower, &500_000, &token);
    }

    #[test]
    fn test_whitelist_enabled_non_whitelisted_voucher_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);

        let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        token_client.mint(&voucher, &1_000_000);
        token_client.mint(&contract_id, &1_000_000);

        client.enable_voucher_whitelist(&Vec::from_array(&env, [admin.clone()]));

        let result = client.try_vouch(&voucher, &borrower, &500_000, &token);
        assert_eq!(result, Err(Ok(ContractError::VoucherNotWhitelisted)));
    }

    #[test]
    fn test_whitelist_enabled_whitelisted_voucher_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);

        let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        token_client.mint(&voucher, &1_000_000);
        token_client.mint(&contract_id, &1_000_000);

        let admins = Vec::from_array(&env, [admin.clone()]);
        client.enable_voucher_whitelist(&admins);
        client.add_voucher_to_whitelist(&admins, &voucher);

        client.vouch(&voucher, &borrower, &500_000, &token);
    }

    #[test]
    fn test_remove_voucher_from_whitelist_blocks_vouch() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);

        let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        token_client.mint(&voucher, &1_000_000);
        token_client.mint(&contract_id, &1_000_000);

        let admins = Vec::from_array(&env, [admin.clone()]);
        client.enable_voucher_whitelist(&admins);
        client.add_voucher_to_whitelist(&admins, &voucher);
        client.remove_voucher_from_whitelist(&admins, &voucher);

        let result = client.try_vouch(&voucher, &borrower, &500_000, &token);
        assert_eq!(result, Err(Ok(ContractError::VoucherNotWhitelisted)));
    }

    #[test]
    fn test_is_voucher_whitelisted_returns_correct_value() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _token) = setup(&env);

        let voucher = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);

        assert!(!client.is_voucher_whitelisted(&voucher));
        client.add_voucher_to_whitelist(&admins, &voucher);
        assert!(client.is_voucher_whitelisted(&voucher));
        client.remove_voucher_from_whitelist(&admins, &voucher);
        assert!(!client.is_voucher_whitelisted(&voucher));
    }

    #[test]
    fn test_disable_voucher_whitelist_allows_all_vouchers() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);

        let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        token_client.mint(&voucher, &1_000_000);
        token_client.mint(&contract_id, &1_000_000);

        let admins = Vec::from_array(&env, [admin.clone()]);
        client.enable_voucher_whitelist(&admins);
        client.disable_voucher_whitelist(&admins);

        // After disabling, non-whitelisted voucher should succeed
        client.vouch(&voucher, &borrower, &500_000, &token);
    }
}
