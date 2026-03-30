#[cfg(test)]
mod min_loan_amount_tests {
    use crate::errors::ContractError;
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::Address as _, token::StellarAssetClient, Address, Env, String, Vec,
    };

    fn setup(env: &Env) -> (Address, Address, Address, Address) {
        let deployer = Address::generate(env);
        let admin = Address::generate(env);
        let token_id = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(env, &contract_id);
        client.initialize(&deployer, &Vec::from_array(env, [admin]), &1, &token_id);
        StellarAssetClient::new(env, &token_id).mint(&contract_id, &10_000_000);
        let voucher = Address::generate(env);
        StellarAssetClient::new(env, &token_id).mint(&voucher, &1_000_000);
        (contract_id, token_id, voucher, Address::generate(env))
    }

    #[test]
    fn test_loan_below_minimum_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 120);
        let (contract_id, token_id, voucher, borrower) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        // Vouch with sufficient stake
        client.vouch(&voucher, &borrower, &500_000, &token_id);

        // Try to request loan with amount below minimum (DEFAULT_MIN_LOAN_AMOUNT = 100_000)
        let result = client.try_request_loan(
            &borrower,
            &1, // 1 stroop - below minimum
            &500_000,
            &String::from_str(&env, "test loan"),
            &token_id,
        );
        assert_eq!(result, Err(Ok(ContractError::LoanBelowMinAmount)));
    }

    #[test]
    fn test_loan_at_minimum_accepted() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 120);
        let (contract_id, token_id, voucher, borrower) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        // Vouch with sufficient stake
        client.vouch(&voucher, &borrower, &500_000, &token_id);

        // Request loan with amount exactly at minimum (DEFAULT_MIN_LOAN_AMOUNT = 100_000)
        let result = client.try_request_loan(
            &borrower,
            &100_000, // Exactly at minimum
            &500_000,
            &String::from_str(&env, "test loan"),
            &token_id,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_loan_just_below_minimum_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 120);
        let (contract_id, token_id, voucher, borrower) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        // Vouch with sufficient stake
        client.vouch(&voucher, &borrower, &500_000, &token_id);

        // Try to request loan with amount just below minimum
        let result = client.try_request_loan(
            &borrower,
            &99_999, // Just below minimum
            &500_000,
            &String::from_str(&env, "test loan"),
            &token_id,
        );
        assert_eq!(result, Err(Ok(ContractError::LoanBelowMinAmount)));
    }
}
