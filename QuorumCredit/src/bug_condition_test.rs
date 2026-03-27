/// Bug Condition Tests — Invalid Token Address Validation
///
/// Verifies that `initialize` rejects a plain account address (not a SEP-41
/// token contract) with a typed `ContractError::InvalidToken` before writing
/// any state, rather than accepting it and causing a runtime panic on the
/// first downstream token operation.
#[cfg(test)]
mod bug_condition_tests {
    use crate::{ContractError, DataKey, QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

    fn vec_of(env: &Env, addr: &Address) -> Vec<Address> {
        let mut v = Vec::new(env);
        v.push_back(addr.clone());
        v
    }

    /// initialize with a plain account address as token must return InvalidToken.
    #[test]
    fn test_initialize_with_invalid_token_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = vec_of(&env, &admin);

        // Plain account address — does NOT implement the SEP-41 token interface.
        let invalid_token = Address::generate(&env);

        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        let result = client.try_initialize(&deployer, &admins, &1, &invalid_token);
        assert!(
            result.is_err(),
            "initialize must reject a plain account address as token"
        );

        // No state must have been written — the call must have been atomic.
        assert!(
            !env.as_contract(&contract_id, || {
                env.storage().instance().has(&DataKey::Config)
            }),
            "DataKey::Config must not be stored after a rejected initialize call"
        );
    }

    /// initialize with a plain account address must return the typed InvalidToken error.
    #[test]
    fn test_initialize_invalid_token_returns_typed_error() {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = vec_of(&env, &admin);
        let invalid_token = Address::generate(&env);

        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        let result = client.try_initialize(&deployer, &admins, &1, &invalid_token);
        assert_eq!(
            result,
            Err(Ok(ContractError::InvalidToken)),
            "expected ContractError::InvalidToken for a non-token address"
        );
    }

    /// After a rejected initialize, a subsequent valid initialize must succeed.
    /// This confirms the contract state is clean after the failed call.
    #[test]
    fn test_valid_initialize_succeeds_after_rejected_one() {
        let env = Env::default();
        env.mock_all_auths();

        use soroban_sdk::token::StellarAssetClient;

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = vec_of(&env, &admin);
        let invalid_token = Address::generate(&env);

        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        // First call with invalid token — must fail.
        let _ = client.try_initialize(&deployer, &admins, &1, &invalid_token);

        // Second call with a real SEP-41 token — must succeed.
        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        StellarAssetClient::new(&env, &token_id.address()).mint(&contract_id, &0);
        client.initialize(&deployer, &admins, &1, &token_id.address());

        assert!(
            env.as_contract(&contract_id, || {
                env.storage().instance().has(&DataKey::Config)
            }),
            "DataKey::Config must be stored after a valid initialize call"
        );
    }
}
