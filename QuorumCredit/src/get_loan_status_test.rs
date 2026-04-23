#[cfg(test)]
mod tests {
    use crate::types::LoanStatus;
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

    #[test]
    fn test_get_loan_status_none_for_unknown_id() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, _token) = setup(&env);
        assert_eq!(client.get_loan_status(&999), LoanStatus::None);
    }

    #[test]
    fn test_get_loan_status_active_after_disbursal() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);

        let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        token_client.mint(&voucher, &10_000_000);
        token_client.mint(&contract_id, &10_000_000);

        env.ledger().with_mut(|li| li.timestamp = 1000);
        client.vouch(&voucher, &borrower, &5_000_000, &token);

        env.ledger().with_mut(|li| li.timestamp = 1000 + 61);
        client.request_loan(
            &borrower,
            &100_000,
            &1_000_000,
            &String::from_str(&env, "test"),
            &token,
        );

        // loan_id starts at 1
        assert_eq!(client.get_loan_status(&1), LoanStatus::Active);
    }

    #[test]
    fn test_get_loan_status_repaid_after_repayment() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, token) = setup(&env);
        let contract_id = client.address.clone();

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);

        let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        token_client.mint(&voucher, &10_000_000);
        // Fund contract with enough for loan + yield
        token_client.mint(&contract_id, &20_000_000);

        env.ledger().with_mut(|li| li.timestamp = 1000);
        client.vouch(&voucher, &borrower, &5_000_000, &token);

        env.ledger().with_mut(|li| li.timestamp = 1000 + 61);
        client.request_loan(
            &borrower,
            &100_000,
            &1_000_000,
            &String::from_str(&env, "test"),
            &token,
        );

        // Repay: principal + yield (2% of 100_000 = 2_000)
        token_client.mint(&borrower, &102_000);
        client.repay(&borrower, &102_000);

        assert_eq!(client.get_loan_status(&1), LoanStatus::Repaid);
    }
}
