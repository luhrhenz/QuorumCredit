/// Loan Record Overwrite Protection Tests
///
/// Verifies that request_loan blocks attempts to overwrite existing active loan records.
/// This prevents an attacker from erasing active loan history by requesting a new loan.
#[cfg(test)]
mod loan_overwrite_protection_tests {
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

        // Advance past MIN_VOUCH_AGE so vouches are eligible.
        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup {
            env,
            client,
            token_id: token_id.address(),
        }
    }

    fn do_vouch(s: &Setup, voucher: &Address, borrower: &Address, stake: i128) {
        StellarAssetClient::new(&s.env, &s.token_id).mint(voucher, &stake);
        s.client.vouch(voucher, borrower, &stake, &s.token_id);
    }

    /// Verify that request_loan blocks when an active loan already exists.
    /// This prevents loan record overwrite attacks.
    #[test]
    #[should_panic(expected = "borrower already has an active loan")]
    fn test_request_loan_blocked_when_active_loan_exists() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        // Setup: vouch for borrower
        do_vouch(&s, &voucher, &borrower, 1_000_000);

        // First loan request - should succeed
        s.client.request_loan(
            &borrower,
            &100_000,
            &500_000,
            &String::from_str(&s.env, "first loan"),
            &s.token_id,
        );

        // Verify loan is active
        let loan = s.client.get_loan(&borrower).expect("loan should exist");
        assert_eq!(loan.status, crate::LoanStatus::Active);

        // Second loan request while first is active - must panic
        // This prevents overwriting the active loan record
        s.client.request_loan(
            &borrower,
            &200_000,
            &500_000,
            &String::from_str(&s.env, "attack attempt"),
            &s.token_id,
        );
    }

    /// Verify that the original loan data is preserved when overwrite is attempted.
    #[test]
    fn test_loan_data_preserved_on_overwrite_attempt() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        do_vouch(&s, &voucher, &borrower, 1_000_000);

        // First loan
        s.client.request_loan(
            &borrower,
            &100_000,
            &500_000,
            &String::from_str(&s.env, "original loan"),
            &s.token_id,
        );

        let original_loan = s.client.get_loan(&borrower).expect("loan should exist");
        let original_id = original_loan.id;
        let original_amount = original_loan.amount;

        // Attempt second loan - should fail
        let result = s.client.try_request_loan(
            &borrower,
            &200_000,
            &500_000,
            &String::from_str(&s.env, "attack attempt"),
            &s.token_id,
        );
        assert!(result.is_err(), "second loan request must fail");

        // Verify original loan is unchanged
        let loan_after = s.client.get_loan(&borrower).expect("loan should still exist");
        assert_eq!(loan_after.id, original_id, "loan ID should not change");
        assert_eq!(loan_after.amount, original_amount, "loan amount should not change");
        assert_eq!(loan_after.status, crate::LoanStatus::Active, "loan should still be active");
    }
}
