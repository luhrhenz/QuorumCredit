/// Slash Authorization Tests
///
/// Verifies that slash via governance (vote_slash) requires the caller to be
/// an active voucher for the borrower, and that a non-voucher is rejected.
/// Also verifies that a legitimate voucher holding majority stake can trigger
/// an auto-slash when quorum is reached.
#[cfg(test)]
mod slash_auth_tests {
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
            admin: admin.clone(),
            admin_vec: admins,
            token_id: token_id.address(),
        }
    }

    fn do_vouch(s: &Setup, voucher: &Address, borrower: &Address, stake: i128) {
        StellarAssetClient::new(&s.env, &s.token_id).mint(voucher, &stake);
        s.client.vouch(voucher, borrower, &stake, &s.token_id);
    }

    fn do_loan(s: &Setup, borrower: &Address) {
        s.client.request_loan(
            borrower,
            &100_000,
            &500_000,
            &String::from_str(&s.env, "test"),
            &s.token_id,
        );
    }

    /// A non-voucher calling vote_slash must be rejected with VoucherNotFound.
    #[test]
    fn test_slash_rejected_when_called_by_non_voucher() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);
        let outsider = Address::generate(&s.env);

        do_vouch(&s, &voucher, &borrower, 1_000_000);
        do_loan(&s, &borrower);

        let result = s.client.try_vote_slash(&outsider, &borrower, &true);
        assert!(
            result.is_err(),
            "vote_slash must be rejected when called by a non-voucher"
        );
    }

    /// A voucher holding 100% of stake triggers auto-slash when quorum is met.
    /// After slash the loan status must be Defaulted.
    #[test]
    fn test_slash_succeeds_when_voucher_reaches_quorum() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        // Set quorum to 1 bps so a single voucher vote triggers slash immediately.
        s.client.set_slash_vote_quorum(&s.admin_vec, &1);

        do_vouch(&s, &voucher, &borrower, 1_000_000);
        do_loan(&s, &borrower);

        s.client.vote_slash(&voucher, &borrower, &true);

        // Loan must now be defaulted.
        assert_eq!(
            s.client.loan_status(&borrower),
            crate::LoanStatus::Defaulted,
            "loan should be Defaulted after slash quorum reached"
        );
    }
}
