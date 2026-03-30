/// Slash Calculation Precision Test - Small Stakes (Issue #137)
///
/// Verifies slash_amount = stake * slash_bps / 10_000 handles small stakes correctly.
/// Default slash_bps=5000 (50%). i128 integer division truncates toward zero.
///
/// Expected for small stakes:
/// - stake=1: slash_amount=1*5000/10000=0, remaining=1, treasury+=0
/// - stake=10: slash_amount=10*5000/10000=5, remaining=5, treasury+=5
/// - stake=19: slash_amount=19*5000/10000=9 (trunc), remaining=10, treasury+=9
///
/// Single voucher votes YES (100% stake >=50% quorum) → auto-executes slash.
/// After slash: voucher gets remaining back (net zero change for stake=1),
/// loan marked defaulted, treasury updated correctly.

#[cfg(test)]
mod slash_precision_tests {
    use crate::{QuorumCreditContract, QuorumCreditContractClient, LoanStatus};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Address, Env, String, Vec,
    };

    // Reuse pattern from governance_test.rs
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

        // Fund contract for loan disbursement
        StellarAssetClient::new(&env, &token_id.address()).mint(&contract_id, &10_000_000);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &1, &token_id.address());

        // Advance past MIN_VOUCH_AGE (60s)
        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup {
            env,
            client,
            token_id: token_id.address(),
        }
    }

    fn do_vouch(s: &Setup, voucher: &Address, borrower: &Address, stake: i128) {
        let token_client = StellarAssetClient::new(&s.env, &s.token_id);
        token_client.mint(voucher, &stake);
        s.client.vouch(voucher, borrower, &stake, &s.token_id);
    }

    fn do_loan(s: &Setup, borrower: &Address, amount: i128, threshold: i128) {
        s.client.request_loan(
            borrower,
            &amount,
            &threshold,
            &String::from_str(&s.env, "precision test"),
            &s.token_id,
        );
    }

    #[test]
    fn test_slash_stake_1_stroop_zero_slash() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        let initial_balance = 100; // Arbitrary initial voucher balance
        let token_client = StellarAssetClient::new(&s.env, &s.token_id);
        token_client.mint(&voucher, &initial_balance);

        let stake = 1_i128;
        let expected_treasury_delta = 0_i128;

        // 1. Vouch
        do_vouch(&s, &voucher, &borrower, stake);

        // Verify stake transferred in
        let after_vouch_balance = token_client.balance(&voucher);
        assert_eq!(after_vouch_balance, initial_balance - stake);

        // 2. Request loan (threshold=stake, so min_vouchers=1 ok)
        do_loan(&s, &borrower, 1, stake);

        // 3. Single voucher votes YES → 100% >= 50% quorum → auto-slash
        s.client.vote_slash(&voucher, &borrower, &true);

        // 4. Assertions
        // Loan defaulted
        assert_eq!(s.client.loan_status(&borrower), LoanStatus::Defaulted);

        // Vote executed
        let vote = s.client.get_slash_vote(&borrower).unwrap();
        assert!(vote.executed);

        // Voucher balance: initial - stake + remaining = initial (net zero change)
        let final_balance = token_client.balance(&voucher);
        assert_eq!(final_balance, initial_balance);

        // Treasury unchanged (slash=0)
        assert_eq!(s.client.get_slash_treasury_balance(), expected_treasury_delta);
    }

    #[test]
    fn test_slash_stake_10_stroops() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        let token_client = StellarAssetClient::new(&s.env, &s.token_id);

        let stake = 10_i128;
        let expected_slash = 5_i128; // 10 * 5000 / 10000 = 5
        let expected_remaining = 5_i128;

        do_vouch(&s, &voucher, &borrower, stake);
        do_loan(&s, &borrower, 1, stake);
        s.client.vote_slash(&voucher, &borrower, &true);

        // Loan defaulted
        assert_eq!(s.client.loan_status(&borrower), LoanStatus::Defaulted);

        // Treasury increased by slash amount
        assert_eq!(s.client.get_slash_treasury_balance(), expected_slash);

        // Final balance = initial + remaining - stake? Wait, initial=0, minted stake, so final=remaining
        let final_balance = token_client.balance(&voucher);
        assert_eq!(final_balance, expected_remaining);
    }

    #[test]
    fn test_slash_stake_19_stroops_truncation() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        let token_client = StellarAssetClient::new(&s.env, &s.token_id);

        let stake = 19_i128;
        let expected_slash = 9_i128; // 19 * 5000 / 10000 = 95000 / 10000 = 9 (trunc)
        let expected_remaining = 10_i128;

        do_vouch(&s, &voucher, &borrower, stake);
        do_loan(&s, &borrower, 1, stake);
        s.client.vote_slash(&voucher, &borrower, &true);

        assert_eq!(s.client.loan_status(&borrower), LoanStatus::Defaulted);
        assert_eq!(s.client.get_slash_treasury_balance(), expected_slash);

        let final_balance = token_client.balance(&voucher);
        assert_eq!(final_balance, expected_remaining);
    }
}

