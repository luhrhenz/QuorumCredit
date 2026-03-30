/// Double Repay Panic Tests
///
/// Verifies that calling repay twice panics on the second call with
/// "loan already repaid" message.
#[cfg(test)]
mod double_repay_tests {
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{StellarAssetClient, TokenClient},
        Address, Env, String, Vec,
    };

    struct Setup {
        env: Env,
        client: QuorumCreditContractClient<'static>,
        token: Address,
    }

    fn setup() -> Setup {
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

        Setup { env, client, token: token_id.address() }
    }

    fn do_vouch(s: &Setup, voucher: &Address, borrower: &Address, stake: i128) {
        StellarAssetClient::new(&s.env, &s.token).mint(voucher, &stake);
        s.client.vouch(voucher, borrower, &stake, &s.token);
        
        // Advance time past MIN_VOUCH_AGE (60s) so the vouch is eligible.
        s.env.ledger().with_mut(|l| l.timestamp += 61);
    }

    fn do_loan(s: &Setup, borrower: &Address, amount: i128) {
        s.client.request_loan(
            borrower,
            &amount,
            &amount, // threshold equal to amount for simplicity
            &String::from_str(&s.env, "test"),
            &s.token,
        );
    }

    /// Calling repay twice must return AlreadyRepaid error on the second call.
    #[test]
    fn test_repay_panics_when_loan_already_repaid() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        let stake: i128 = 1_000_000;
        let loan_amount: i128 = 500_000;

        // Setup vouch and fund contract
        do_vouch(&s, &voucher, &borrower, stake);
        StellarAssetClient::new(&s.env, &s.token).mint(&s.client.address, &loan_amount);

        // Request and disburse loan
        do_loan(&s, &borrower, loan_amount);

        // Mint tokens to borrower for repayment
        let yield_amount = loan_amount * 200 / 10_000; // 2% yield
        let total_owed = loan_amount + yield_amount;
        StellarAssetClient::new(&s.env, &s.token).mint(&borrower, &total_owed);

        // First repay - should succeed
        s.client.repay(&borrower, &total_owed);

        // Verify loan is marked as repaid
        let loan = s.client.get_loan(&borrower);
        assert!(loan.is_some(), "loan should exist");
        assert_eq!(loan.unwrap().status, crate::LoanStatus::Repaid);

        // Second repay - must return AlreadyRepaid error
        let result = s.client.try_repay(&borrower, &total_owed);
        assert_eq!(result, Err(Ok(crate::ContractError::AlreadyRepaid)));
    }

    /// Verify that partial repayments are allowed until loan is fully repaid
    #[test]
    fn test_partial_repayments_allowed_until_fully_repaid() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        let stake: i128 = 1_000_000;
        let loan_amount: i128 = 500_000;

        // Setup vouch and fund contract
        do_vouch(&s, &voucher, &borrower, stake);
        StellarAssetClient::new(&s.env, &s.token).mint(&s.client.address, &loan_amount);

        // Request and disburse loan
        do_loan(&s, &borrower, loan_amount);

        // Mint tokens to borrower for repayment
        let yield_amount = loan_amount * 200 / 10_000; // 2% yield
        let total_owed = loan_amount + yield_amount;
        StellarAssetClient::new(&s.env, &s.token).mint(&borrower, &total_owed);

        // Partial repayment - should succeed
        let partial_payment = total_owed / 2;
        s.client.repay(&borrower, &partial_payment);

        // Verify loan is still active
        let loan = s.client.get_loan(&borrower);
        assert!(loan.is_some(), "loan should exist");
        assert_eq!(loan.unwrap().status, crate::LoanStatus::Active);

        // Final repayment - should succeed
        let remaining_payment = total_owed - partial_payment;
        s.client.repay(&borrower, &remaining_payment);

        // Verify loan is now repaid
        let loan = s.client.get_loan(&borrower);
        assert!(loan.is_some(), "loan should exist");
        assert_eq!(loan.unwrap().status, crate::LoanStatus::Repaid);
    }
}
