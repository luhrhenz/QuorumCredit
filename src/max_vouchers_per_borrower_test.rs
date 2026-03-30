#[cfg(test)]
mod tests {
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{StellarAssetClient as TokenAdminClient, TokenClient},
        Address, Env, String, Vec,
    };

    fn create_token_contract<'a>(
        env: &Env,
        admin: &Address,
    ) -> (TokenClient<'a>, TokenAdminClient<'a>) {
        let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
        (
            TokenClient::new(env, &contract_address.address()),
            TokenAdminClient::new(env, &contract_address.address()),
        )
    }

    #[test]
    fn test_max_vouchers_per_borrower_enforcement() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });

        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);

        let (token_client, token_admin) = create_token_contract(&env, &admin);
        let token = token_client.address.clone();

        client.initialize(&deployer, &admins, &1, &token);

        // Set max vouchers per borrower to 3 for testing
        client.set_max_vouchers_per_borrower(&Vec::from_array(&env, [admin.clone()]), &3);

        // Verify the setting was applied
        assert_eq!(client.get_max_vouchers_per_borrower(), 3);

        let borrower = Address::generate(&env);

        // Mint tokens to vouchers
        let voucher1 = Address::generate(&env);
        let voucher2 = Address::generate(&env);
        let voucher3 = Address::generate(&env);
        let voucher4 = Address::generate(&env);

        token_admin.mint(&voucher1, &1_000_000);
        token_admin.mint(&voucher2, &1_000_000);
        token_admin.mint(&voucher3, &1_000_000);
        token_admin.mint(&voucher4, &1_000_000);

        // First 3 vouches should succeed
        client.vouch(&voucher1, &borrower, &100_000, &token);
        client.vouch(&voucher2, &borrower, &100_000, &token);
        client.vouch(&voucher3, &borrower, &100_000, &token);

        // 4th vouch should fail with MaxVouchersPerBorrowerExceeded error
        let result = client.try_vouch(&voucher4, &borrower, &100_000, &token);
        assert!(result.is_err());
        assert_eq!(
            result.err(),
            Some(Ok(crate::ContractError::MaxVouchersPerBorrowerExceeded))
        );

        // Verify only 3 vouches exist
        let vouches = client.get_vouches(&borrower).unwrap();
        assert_eq!(vouches.len(), 3);
    }

    #[test]
    fn test_default_max_vouchers_per_borrower() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);

        let (token_client, _) = create_token_contract(&env, &admin);
        let token = token_client.address.clone();

        client.initialize(&deployer, &admins, &1, &token);

        // Should return default value (50)
        assert_eq!(
            client.get_max_vouchers_per_borrower(),
            crate::types::DEFAULT_MAX_VOUCHERS_PER_BORROWER
        );
    }

    #[test]
    fn test_admin_can_update_max_vouchers_per_borrower() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);

        let (token_client, _) = create_token_contract(&env, &admin);
        let token = token_client.address.clone();

        client.initialize(&deployer, &admins, &1, &token);

        // Update to 100
        client.set_max_vouchers_per_borrower(&Vec::from_array(&env, [admin.clone()]), &100);
        assert_eq!(client.get_max_vouchers_per_borrower(), 100);

        // Update to 25
        client.set_max_vouchers_per_borrower(&Vec::from_array(&env, [admin.clone()]), &25);
        assert_eq!(client.get_max_vouchers_per_borrower(), 25);
    }

    #[test]
    #[should_panic(expected = "max_vouchers_per_borrower must be greater than zero")]
    fn test_cannot_set_zero_max_vouchers_per_borrower() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);

        let (token_client, _) = create_token_contract(&env, &admin);
        let token = token_client.address.clone();

        client.initialize(&deployer, &admins, &1, &token);

        // Should panic
        client.set_max_vouchers_per_borrower(&Vec::from_array(&env, [admin.clone()]), &0);
    }

    #[test]
    fn test_voucher_can_be_added_after_withdrawal() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });

        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);

        let (token_client, token_admin) = create_token_contract(&env, &admin);
        let token = token_client.address.clone();

        client.initialize(&deployer, &admins, &1, &token);

        // Set max vouchers per borrower to 2
        client.set_max_vouchers_per_borrower(&Vec::from_array(&env, [admin.clone()]), &2);

        let borrower = Address::generate(&env);
        let voucher1 = Address::generate(&env);
        let voucher2 = Address::generate(&env);
        let voucher3 = Address::generate(&env);

        token_admin.mint(&voucher1, &1_000_000);
        token_admin.mint(&voucher2, &1_000_000);
        token_admin.mint(&voucher3, &1_000_000);

        // Add 2 vouches (at limit)
        client.vouch(&voucher1, &borrower, &100_000, &token);
        client.vouch(&voucher2, &borrower, &100_000, &token);

        // 3rd vouch should fail
        let result = client.try_vouch(&voucher3, &borrower, &100_000, &token);
        assert!(result.is_err());

        // Withdraw one vouch
        client.withdraw_vouch(&voucher1, &borrower);

        // Now voucher3 should be able to vouch
        client.vouch(&voucher3, &borrower, &100_000, &token);

        let vouches = client.get_vouches(&borrower).unwrap();
        assert_eq!(vouches.len(), 2);
    }
}
