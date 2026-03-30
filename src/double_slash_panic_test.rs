/// Double-Slash Panic Tests
///
/// Bug condition: calling slash twice on the same borrower should panic with
/// "already defaulted" on the second call.
///
/// Property 1 (Bug Condition): second vote_slash on an already-defaulted loan
///   panics with "already defaulted".
/// Property 2 (Preservation): first slash still marks loan defaulted and slashes
///   voucher stakes; slash on a missing loan still errors.
#[cfg(test)]
mod double_slash_panic_tests {
    use crate::{QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Address, Env, String, Vec,
    };

    struct Setup {
        env: Env,
        client: QuorumCreditContractClient<'static>,
        admin: Address,
        admin_vec: Vec<Address>,
        token: Address,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admin_vec = Vec::from_array(&env, [admin.clone()]);
        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let contract_id = env.register_contract(None, QuorumCreditContract);

        // Fund the contract so it can disburse loans.
        StellarAssetClient::new(&env, &token_id.address()).mint(&contract_id, &10_000_000);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admin_vec, &1, &token_id.address());

        // Advance past MIN_VOUCH_AGE (60 s) so vouches are eligible.
        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup {
            env,
            client,
            admin,
            admin_vec,
            token: token_id.address(),
        }
    }

    /// Set up a voucher + borrower with an active loan, then return both addresses.
    fn setup_active_loan(s: &Setup) -> (Address, Address) {
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);

        StellarAssetClient::new(&s.env, &s.token).mint(&voucher, &1_000_000);
        s.client.vouch(&voucher, &borrower, &1_000_000, &s.token);
        s.client.request_loan(
            &borrower,
            &100_000,
            &500_000,
            &String::from_str(&s.env, "test"),
            &s.token,
        );

        (voucher, borrower)
    }

    // ── Property 1: Bug Condition ─────────────────────────────────────────────

    /// **Property 1: Bug Condition** - Double Slash Panic
    ///
    /// isBugCondition: loan.defaulted == true AND vote_slash called again.
    ///
    /// Run on UNFIXED code: EXPECTED to FAIL (second call does not panic with
    /// "already defaulted" — confirms the bug exists).
    /// After fix: EXPECTED to PASS.
    #[test]
    #[should_panic(expected = "already defaulted")]
    fn test_double_slash_panics_with_already_defaulted() {
        let s = setup();
        // Set quorum to 1 bps so a single voucher vote triggers slash immediately.
        s.client.set_slash_vote_quorum(&s.admin_vec, &1);

        let (voucher, borrower) = setup_active_loan(&s);

        // First slash — must succeed and mark loan as defaulted.
        s.client.vote_slash(&voucher, &borrower, &true);

        // Second slash — must panic with "already defaulted".
        // On unfixed code this will NOT panic with that message, causing the test to fail.
        s.client.vote_slash(&voucher, &borrower, &true);
    }

    // ── Property 2: Preservation ──────────────────────────────────────────────

    /// **Property 2: Preservation** - First Slash Marks Loan Defaulted
    ///
    /// ¬isBugCondition: loan is active (not yet defaulted).
    /// Observed on unfixed code: first slash marks loan.defaulted = true and
    /// slashes voucher stakes.
    #[test]
    fn test_first_slash_marks_loan_defaulted() {
        let s = setup();
        s.client.set_slash_vote_quorum(&s.admin_vec, &1);

        let (voucher, borrower) = setup_active_loan(&s);

        s.client.vote_slash(&voucher, &borrower, &true);

        let loan = s.client.get_loan(&borrower).expect("loan should exist");
        assert_eq!(
            loan.status,
            crate::LoanStatus::Defaulted,
            "first slash must mark loan as defaulted"
        );
    }

    /// **Property 2: Preservation** - Missing Loan Still Errors
    ///
    /// ¬isBugCondition: no loan exists for the borrower.
    /// Observed on unfixed code: vote_slash returns Err(NoActiveLoan).
    #[test]
    fn test_slash_on_missing_loan_returns_error() {
        let s = setup();
        s.client.set_slash_vote_quorum(&s.admin_vec, &1);

        // Voucher vouches for borrower but no loan is requested.
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        StellarAssetClient::new(&s.env, &s.token).mint(&voucher, &1_000_000);
        s.client.vouch(&voucher, &borrower, &1_000_000, &s.token);

        let result = s.client.try_vote_slash(&voucher, &borrower, &true);
        assert!(
            result.is_err(),
            "slash on a borrower with no loan must return an error"
        );
    }
}
