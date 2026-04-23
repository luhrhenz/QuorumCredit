#[cfg(test)]
mod tests {
    use crate::errors::ContractError;
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

    fn setup(env: &Env) -> (QuorumCreditContractClient, Address) {
        let token_admin = Address::generate(env);
        let token = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(env, &contract_id);
        let deployer = Address::generate(env);
        let admin = Address::generate(env);
        client.initialize(&deployer, &Vec::from_array(env, [admin.clone()]), &1, &token);
        (client, admin)
    }

    #[test]
    fn test_set_min_loan_amount_zero_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let result =
            client.try_set_min_loan_amount(&Vec::from_array(&env, [admin.clone()]), &0);
        assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
    }

    #[test]
    fn test_set_min_loan_amount_negative_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        let result =
            client.try_set_min_loan_amount(&Vec::from_array(&env, [admin.clone()]), &-1);
        assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
    }

    #[test]
    fn test_set_min_loan_amount_positive_accepted() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env);

        client
            .set_min_loan_amount(&Vec::from_array(&env, [admin.clone()]), &500_000);

        let cfg = client.get_config();
        assert_eq!(cfg.min_loan_amount, 500_000);
    }
}
