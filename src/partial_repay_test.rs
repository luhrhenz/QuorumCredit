/// Partial Repayment Test (Issue #376)
///
/// Verify that partial repayment preserves vouches and active loan pointer.
/// Only full repayment should clear these.

#[cfg(test)]
mod partial_repay_tests {
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Address, Env, String, Vec,
    };

    struct Setup {
        env: Env,
        client: QuorumCreditContractClient<'static>,
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

        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup { env, client, token_id: token_id.address() }
    }

    fn do_vouch(s: &Setup, voucher: &Address, borrower: &Address, stake: i128) {
        let token = StellarAssetClient::new(&s.env, &s.token_id);
        token.mint(voucher, &stake);
        s.client.vouch(voucher, borrower, &stake, &s.token_id);
    }

    fn purpose(env: &Env) -> String {
        String::from_str(env, "test loan")
    }

    /// Issue #376: Partial repayment preserves vouches and active loan pointer
    #[test]
    fn test_partial_repayment_preserves_vouches_and_active_loan() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        // Setup: vouch and request loan
        do_vouch(&s, &voucher, &borrower, 500_000);
        s.client.request_loan(&borrower, &100_000, &500_000, &purpose(&s.env), &s.token_id);

        // Verify loan is active
        let loan = s.client.get_loan(&borrower).expect("loan should exist");
        assert_eq!(loan.status, crate::LoanStatus::Active);

        // Verify vouches exist
        let vouches = s.client.get_vouches(&borrower).expect("vouches should exist");
        assert_eq!(vouches.len(), 1);

        // Partial repayment: repay 50_000 (half of principal + yield)
        let token = StellarAssetClient::new(&s.env, &s.token_id);
        token.mint(&borrower, &50_000);
        s.client.repay(&borrower, &50_000);

        // After partial repayment:
        // - Loan should still be Active
        let loan_after = s.client.get_loan(&borrower).expect("loan should still exist");
        assert_eq!(loan_after.status, crate::LoanStatus::Active, "loan should still be active");
        assert_eq!(loan_after.amount_repaid, 50_000, "amount_repaid should be updated");

        // - Vouches should still exist
        let vouches_after = s.client.get_vouches(&borrower).expect("vouches should still exist");
        assert_eq!(vouches_after.len(), 1, "vouches should be preserved");
        assert_eq!(vouches_after.get(0).unwrap().stake, 500_000, "vouch stake should be unchanged");
    }

    /// Issue #376: Full repayment clears vouches and active loan pointer
    #[test]
    fn test_full_repayment_clears_vouches_and_active_loan() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        // Setup: vouch and request loan
        do_vouch(&s, &voucher, &borrower, 500_000);
        s.client.request_loan(&borrower, &100_000, &500_000, &purpose(&s.env), &s.token_id);

        // Verify loan is active
        let loan = s.client.get_loan(&borrower).expect("loan should exist");
        assert_eq!(loan.status, crate::LoanStatus::Active);

        // Verify vouches exist
        let vouches = s.client.get_vouches(&borrower).expect("vouches should exist");
        assert_eq!(vouches.len(), 1);

        // Full repayment: repay principal + yield (100_000 + 2_000 = 102_000)
        let token = StellarAssetClient::new(&s.env, &s.token_id);
        token.mint(&borrower, &102_000);
        s.client.repay(&borrower, &102_000);

        // After full repayment:
        // - Loan should be Repaid
        let loan_after = s.client.get_loan(&borrower).expect("loan should still exist");
        assert_eq!(loan_after.status, crate::LoanStatus::Repaid, "loan should be marked repaid");

        // - Vouches should be cleared
        let vouches_after = s.client.get_vouches(&borrower);
        assert!(vouches_after.is_none(), "vouches should be cleared after full repayment");
    }
}
