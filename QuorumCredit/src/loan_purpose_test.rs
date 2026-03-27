#[cfg(test)]
mod loan_purpose_tests {
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Address, Env, String, Vec,
    };

    fn setup() -> (Env, QuorumCreditContractClient<'static>, Address, Address) {
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

        env.ledger().with_mut(|l| l.timestamp = 120);

        let voucher = Address::generate(&env);
        let borrower = Address::generate(&env);
        StellarAssetClient::new(&env, &token_id.address()).mint(&voucher, &1_000_000);
        client.vouch(&voucher, &borrower, &1_000_000, &token_id.address());

        (env, client, borrower, token_id.address())
    }

    #[test]
    fn test_loan_purpose_stored_and_retrieved() {
        let (env, client, borrower, token) = setup();
        let purpose = String::from_str(&env, "school fees");

        client.request_loan(&borrower, &100_000, &500_000, &purpose, &token);

        let loan = client.get_loan(&borrower).unwrap();
        assert_eq!(loan.loan_purpose, purpose);
    }

    #[test]
    fn test_loan_purpose_empty_string_allowed() {
        let (env, client, borrower, token) = setup();
        let purpose = String::from_str(&env, "");

        client.request_loan(&borrower, &100_000, &500_000, &purpose, &token);

        let loan = client.get_loan(&borrower).unwrap();
        assert_eq!(loan.loan_purpose, purpose);
    }
}
