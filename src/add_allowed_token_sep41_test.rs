/// Bug fix: add_allowed_token must validate the token implements SEP-41.
///
/// A plain account address passed to add_allowed_token would previously be
/// accepted without validation, causing panics on any vouch or loan that
/// used that token. This test asserts the fix is in place.
#[cfg(test)]
mod add_allowed_token_sep41_tests {
    use crate::{ContractError, QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::Address as _,
        token::StellarAssetClient,
        Address, Env, Vec,
    };

    struct Setup {
        env: Env,
        client: QuorumCreditContractClient<'static>,
        admin: Address,
        token_id: Address,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);

        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let contract_id = env.register_contract(None, QuorumCreditContract);

        StellarAssetClient::new(&env, &token_id.address()).mint(&contract_id, &10_000_000);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &1, &token_id.address());

        Setup { env, client, admin, token_id: token_id.address() }
    }

    /// A plain account address (not a SEP-41 contract) must be rejected.
    #[test]
    fn test_add_allowed_token_rejects_non_token_address() {
        let s = setup();
        let admins = Vec::from_array(&s.env, [s.admin.clone()]);
        let invalid_token = Address::generate(&s.env);

        let result = s.client.try_add_allowed_token(&admins, &invalid_token);
        assert!(
            result.is_err(),
            "add_allowed_token must reject a plain account address"
        );
    }

    /// A valid SEP-41 token must be accepted.
    #[test]
    fn test_add_allowed_token_accepts_valid_sep41_token() {
        let s = setup();
        let admins = Vec::from_array(&s.env, [s.admin.clone()]);

        let usdc = env_register_token(&s.env, &s.admin, &s.client.address);
        s.client.add_allowed_token(&admins, &usdc);

        let cfg = s.client.get_config();
        assert!(
            cfg.allowed_tokens.iter().any(|t| t == usdc),
            "valid SEP-41 token should be in allowed_tokens after add"
        );
    }

    /// Adding a duplicate token must be rejected.
    #[test]
    fn test_add_allowed_token_rejects_duplicate() {
        let s = setup();
        let admins = Vec::from_array(&s.env, [s.admin.clone()]);

        let usdc = env_register_token(&s.env, &s.admin, &s.client.address);
        s.client.add_allowed_token(&admins, &usdc);

        // Try to add the same token again
        let result = s.client.try_add_allowed_token(&admins, &usdc);
        assert_eq!(result, Err(Ok(ContractError::DuplicateToken)));
    }

    fn env_register_token(env: &Env, admin: &Address, contract_id: &Address) -> Address {
        let token = env.register_stellar_asset_contract_v2(admin.clone());
        StellarAssetClient::new(env, &token.address()).mint(contract_id, &0);
        token.address()
    }
}
