/// get_vouches Empty State Tests
///
/// Verifies that get_vouches returns None for an address that has never been vouched for.
#[cfg(test)]
mod get_vouches_empty_tests {
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env, Vec};

    fn setup() -> (Env, QuorumCreditContractClient<'static>) {
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

        (env, client)
    }

    /// Calling get_vouches on a fresh address with no vouches should return None.
    #[test]
    fn test_get_vouches_returns_none_for_address_with_no_vouches() {
        let (env, client) = setup();
        let fresh = Address::generate(&env);

        let result = client.get_vouches(&fresh);
        assert!(result.is_none(), "get_vouches should return None for an address with no vouches");
    }
}
