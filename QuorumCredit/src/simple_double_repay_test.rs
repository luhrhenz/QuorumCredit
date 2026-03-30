/// Simple test to verify double repay panic functionality
#[cfg(test)]
mod simple_double_repay_test {
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{StellarAssetClient, TokenClient},
        Address, Env, String, Vec,
    };

    #[test]
    fn test_double_repay_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);
        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let contract_id = env.register_contract(None, QuorumCreditContract);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &1, &token_id.address());

        env.ledger().with_mut(|l| l.timestamp = 120);

        let borrower = Address::generate(&env);
        let voucher = Address::generate(&env);

        let stake: i128 = 1_000_000;
        let loan_amount: i128 = 500_000;

        // Setup vouch and fund contract
        StellarAssetClient::new(&env, &token_id.address()).mint(&voucher, &stake);
        client.vouch(&voucher, &borrower, &stake, &token_id.address());
        
        // Advance time past MIN_VOUCH_AGE (60s) so the vouch is eligible.
        env.ledger().with_mut(|l| l.timestamp += 61);

        // Fund contract with the loan amount
        StellarAssetClient::new(&env, &token_id.address()).mint(&client.address, &loan_amount);

        // Request and disburse loan
        client.request_loan(
            &borrower,
            &loan_amount,
            &stake,
            &String::from_str(&env, "test"),
            &token_id.address(),
        );

        // Mint tokens to borrower for repayment
        let yield_amount = loan_amount * 200 / 10_000; // 2% yield
        let total_owed = loan_amount + yield_amount;
        StellarAssetClient::new(&env, &token_id.address()).mint(&borrower, &total_owed);

        // First repay - should succeed
        client.repay(&borrower, &total_owed);

        // Verify loan is marked as repaid
        let loan = client.get_loan(&borrower);
        assert!(loan.is_some(), "loan should exist");
        assert_eq!(loan.unwrap().status, crate::LoanStatus::Repaid);

        // Second repay - must return AlreadyRepaid error
        let result = client.try_repay(&borrower, &total_owed);
        assert_eq!(result, Err(Ok(crate::ContractError::AlreadyRepaid)));
    }
}
