#[cfg(test)]
mod multi_asset_tests {
    use crate::{ContractError, QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Address, Env, String, Vec,
    };

    struct Setup {
        env: Env,
        client: QuorumCreditContractClient<'static>,
        xlm: Address,
        usdc: Address,
        admin: Address,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);

        let xlm_id = env.register_stellar_asset_contract_v2(admin.clone());
        let usdc_id = env.register_stellar_asset_contract_v2(admin.clone());
        let contract_id = env.register_contract(None, QuorumCreditContract);

        StellarAssetClient::new(&env, &xlm_id.address()).mint(&contract_id, &10_000_000);
        StellarAssetClient::new(&env, &usdc_id.address()).mint(&contract_id, &10_000_000);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &1, &xlm_id.address());
        client.add_allowed_token(&admins, &usdc_id.address());

        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup { env, client, xlm: xlm_id.address(), usdc: usdc_id.address(), admin }
    }

    fn purpose(env: &Env) -> String {
        String::from_str(env, "test")
    }

    #[test]
    fn test_usdc_loan_token_address_stored() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);

        StellarAssetClient::new(&s.env, &s.usdc).mint(&voucher, &1_000_000);
        s.client.vouch(&voucher, &borrower, &1_000_000, &s.usdc);
        // Advance time past MIN_VOUCH_AGE (60s) so the vouch is eligible
        s.env.ledger().with_mut(|l| l.timestamp += 61);
        s.client.request_loan(&borrower, &100_000, &500_000, &purpose(&s.env), &s.usdc);

        let loan = s.client.get_loan(&borrower).unwrap();
        assert_eq!(loan.token_address, s.usdc);
    }

    /// XLM vouches must not count toward a USDC loan threshold.
    #[test]
    fn test_xlm_vouches_dont_count_for_usdc_loan() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);

        StellarAssetClient::new(&s.env, &s.xlm).mint(&voucher, &1_000_000);
        s.client.vouch(&voucher, &borrower, &1_000_000, &s.xlm);
        // Advance time past MIN_VOUCH_AGE (60s) so the vouch is eligible
        s.env.ledger().with_mut(|l| l.timestamp += 61);

        let result = s.client.try_request_loan(&borrower, &100_000, &500_000, &purpose(&s.env), &s.usdc);
        assert!(result.is_err());
    }

    #[test]
    fn test_vouch_with_disallowed_token_rejected() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let random_token = Address::generate(&s.env);

        let result = s.client.try_vouch(&voucher, &borrower, &100_000, &random_token);
        assert_eq!(result, Err(Ok(ContractError::InvalidToken)));
    }

    #[test]
    fn test_request_loan_with_disallowed_token_rejected() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let random_token = Address::generate(&s.env);

        let result = s.client.try_request_loan(&borrower, &100_000, &500_000, &purpose(&s.env), &random_token);
        assert_eq!(result, Err(Ok(ContractError::InvalidToken)));
    }

    #[test]
    fn test_remove_allowed_token_blocks_vouch() {
        let s = setup();
        let admins = Vec::from_array(&s.env, [s.admin.clone()]);
        s.client.remove_allowed_token(&admins, &s.usdc);

        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let result = s.client.try_vouch(&voucher, &borrower, &100_000, &s.usdc);
        assert_eq!(result, Err(Ok(ContractError::InvalidToken)));
    }

    #[test]
    fn test_primary_token_always_allowed() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);

        StellarAssetClient::new(&s.env, &s.xlm).mint(&voucher, &1_000_000);
        s.client.vouch(&voucher, &borrower, &1_000_000, &s.xlm);
        // Advance time past MIN_VOUCH_AGE (60s) so the vouch is eligible
        s.env.ledger().with_mut(|l| l.timestamp += 61);
        s.client.request_loan(&borrower, &100_000, &500_000, &purpose(&s.env), &s.xlm);

        let loan = s.client.get_loan(&borrower).unwrap();
        assert_eq!(loan.token_address, s.xlm);
    }
}
