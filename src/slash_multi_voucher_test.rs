/// Slash Multi-Voucher Test (Issue #139)
///
/// 1. 3 vouchers: stakes 301_000, 200_000, 100_001 (total=601_001)
/// 2. One voucher votes YES (50% stake >= 50% quorum) → auto-slash
/// 3. slash_amount = stake * 5000 / 10000 (trunc):
///    v1: 301000 → slash=150500, remaining=150500
///    v2: 200000 → 100000, 100000
///    v3: 100001 → 50000, 50001 (trunc)
/// 4. treasury += 300500
/// 5. Each final_balance = initial_mint - slash_amount (net 50% loss)

#[cfg(test)]
mod slash_multi_voucher_tests {
    use crate::{LoanStatus, QuorumCreditContract, QuorumCreditContractClient};
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

        // Fund contract
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
        let token = StellarAssetClient::new(&s.env, &s.token_id);
        token.mint(voucher, &stake);
        s.client.vouch(voucher, borrower, &stake, &s.token_id);
    }

    fn purpose(env: &Env) -> String {
        String::from_str(env, "slash multi-voucher test")
    }

    #[test]
    fn test_slash_multi_voucher_all_lose_50_percent() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher1 = Address::generate(&s.env); // 301_000 → slash=150_500
        let voucher2 = Address::generate(&s.env); // 200_000 → 100_000
        let voucher3 = Address::generate(&s.env); // 100_001 → 50_000

        let stakes = [301_000i128, 200_000i128, 100_001i128];
        let vouchers = [&voucher1, &voucher2, &voucher3];
        let total_stake = stakes.iter().sum::<i128>();
        let loan_amount = 100_000i128;
        let slash_bps = 5_000i128;
        let expected_slash1 = stakes[0] * slash_bps / 10_000; // 150500
        let expected_slash2 = stakes[1] * slash_bps / 10_000; // 100000
        let expected_slash3 = stakes[2] * slash_bps / 10_000; // 50000 (trunc)
        let total_slash = expected_slash1 + expected_slash2 + expected_slash3; // 300500
        let token = StellarAssetClient::new(&s.env, &s.token_id);

        // Record initial balances (minted stakes)
        let mut initial_bals = [0i128; 3];
        for i in 0..3 {
            token.mint(vouchers[i], &stakes[i]);
            initial_bals[i] = token.balance(vouchers[i]);
            assert_eq!(initial_bals[i], stakes[i]);
        }

        // 1. Vouch
        do_vouch(&s, vouchers[0], &borrower, stakes[0]);
        do_vouch(&s, vouchers[1], &borrower, stakes[1]);
        do_vouch(&s, vouchers[2], &borrower, stakes[2]);

        // Post-vouch balances = 0
        for i in 0..3 {
            assert_eq!(token.balance(vouchers[i]), 0);
        }

        let vouched = s.client.total_vouched(&borrower).unwrap();
        assert_eq!(vouched, total_stake);

        // 2. Request loan
        s.client.request_loan(
            &borrower,
            &loan_amount,
            &total_stake,
            &purpose(&s.env),
            &s.token_id,
        );

        let loan = s.client.get_loan(&borrower).expect("loan exists");
        assert_eq!(loan.amount, loan_amount);
        assert_eq!(loan.status, crate::LoanStatus::Active);

        // 3. Voucher1 votes YES (~50% stake >= 50% quorum) → auto-slash
        s.client.vote_slash(&voucher1, &borrower, &true);

        // 4. Assertions
        assert_eq!(s.client.loan_status(&borrower), LoanStatus::Defaulted);

        let vote = s.client.get_slash_vote(&borrower).unwrap();
        assert!(vote.executed);

        assert_eq!(s.client.get_slash_treasury_balance(), total_slash);

        // Vouches cleared
        assert!(s.client.get_vouches(&borrower).is_none());

        // Each final balance = initial - slash (net 50% loss)
        assert_eq!(token.balance(&voucher1), initial_bals[0] - expected_slash1);
        assert_eq!(token.balance(&voucher2), initial_bals[1] - expected_slash2);
        assert_eq!(token.balance(&voucher3), initial_bals[2] - expected_slash3);
    }
}
