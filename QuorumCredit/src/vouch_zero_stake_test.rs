#[cfg(test)]
mod vouch_zero_stake_tests {
    use crate::errors::ContractError;
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env, Vec};

    fn setup(env: &Env) -> (Address, Address, Address, Address) {
        let deployer = Address::generate(env);
        let admin = Address::generate(env);
        let token_id = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(env, &contract_id);
        client.initialize(
            &deployer,
            &Vec::from_array(env, [admin]),
            &1,
            &token_id,
        );
        let voucher = Address::generate(env);
        StellarAssetClient::new(env, &token_id).mint(&voucher, &1_000_000);
        (contract_id, token_id, voucher, Address::generate(env))
    }

    #[test]
    fn test_vouch_zero_stake_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_id, voucher, borrower) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        let result = client.try_vouch(&voucher, &borrower, &0, &token_id);
        assert_eq!(result, Err(Ok(ContractError::InsufficientFunds)));
    }
}
