/// Repay Multi-Voucher Yield Distribution Test (Issue #138)
///
/// 1. Add 3 vouchers: stakes 300_000, 200_000, 100_000 (total_stake=600_000)
/// 2. Request loan=100_000 (threshold=600_000)
/// 3. Repay 102_000 (principal + 2% yield=2_000)
/// 4. Assert each: balance == stake + proportional_yield
///    - total_yield=100_000*200/10_000=2_000
///    - v1: 300k/600k → +1_000
///    - v2: 200k/600k → +666 (int div)
///    - v3: 100k/600k → +333

#[cfg(test)]
mod repay_multi_voucher_yield_tests {
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

        // Fund contract sufficiently for loan disbursement + yield payouts
        StellarAssetClient::new(&env, &token_id.address()).mint(&contract_id, &10_000_000);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &1, &token_id.address());

        // Advance past MIN_VOUCH_AGE (60s)
        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup { env, client, token_id: token_id.address() }
    }

    fn do_vouch(s: &Setup, voucher: &Address, borrower: &Address, stake: i128) {
        let token = StellarAssetClient::new(&s.env, &s.token_id);
        token.mint(voucher, &stake);
        s.client.vouch(voucher, borrower, &stake, &s.token_id);
    }

    fn purpose(env: &Env) -> String {
        String::from_str(env, "business expansion")
    }

    #[test]
    fn test_repay_with_multiple_vouchers_all_receive_stake_plus_yield() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher1 = Address::generate(&s.env);  // 300_000 stake → yield=1_000
        let voucher2 = Address::generate(&s.env);  // 200_000 → 666
        let voucher3 = Address::generate(&s.env);  // 100_000 → 333

        let stakes = [300_000i128, 200_000i128, 100_000i128];
        let vouchers = [&voucher1, &voucher2, &voucher3];
        let total_stake = 600_000i128;
        let loan_amount = 100_000i128;
        let yield_bps = 200i128;
        let total_yield = loan_amount * yield_bps / 10_000;  // 2_000
        let total_repay = loan_amount + total_yield;  // 102_000

        let token = StellarAssetClient::new(&s.env, &s.token_id);

        // Record initial balances after mint (== stake, before vouch transfer)
        let initial_bal1 = token.balance(&voucher1);
        let initial_bal2 = token.balance(&voucher2);
        let initial_bal3 = token.balance(&voucher3);
        assert_eq!(initial_bal1, stakes[0]);
        assert_eq!(initial_bal2, stakes[1]);
        assert_eq!(initial_bal3, stakes[2]);

        // 1. Vouch with different stakes
        do_vouch(&s, vouchers[0], &borrower, stakes[0]);
        do_vouch(&s, vouchers[1], &borrower, stakes[1]);
        do_vouch(&s, vouchers[2], &borrower, stakes[2]);

        // Verify total_vouched
        let vouched = s.client.total_vouched(&borrower).unwrap();
        assert_eq!(vouched, total_stake);

        // Post-vouch: balances == 0 (stake transferred to contract)
        assert_eq!(token.balance(&voucher1), 0);
        assert_eq!(token.balance(&voucher2), 0);
        assert_eq!(token.balance(&voucher3), 0);

        // 2. Request loan
        s.client.request_loan(&borrower, &loan_amount, &total_stake, &purpose(&s.env), &s.token_id);

        // Verify loan
        let loan = s.client.get_loan(&borrower).expect("loan should exist");
        assert_eq!(loan.amount, loan_amount);
        assert_eq!(loan.total_yield, total_yield);
        assert_eq!(loan.status, crate::LoanStatus::Active);

        // 3. Fund borrower and repay
        token.mint(&borrower, &total_repay);
        s.client.repay(&borrower, &total_repay);

        // 4. Assertions
        let repaid_loan = s.client.get_loan(&borrower).expect("loan still exists");
        assert_eq!(repaid_loan.status, crate::LoanStatus::Repaid, "loan should be marked repaid");

        // Each voucher receives stake + proportional yield
        // final_balance = 0 (post-vouch) + stake + yield_i == initial + yield_i
        let exp_yield1 = total_yield * stakes[0] / total_stake;  // 1000
        let exp_yield2 = total_yield * stakes[1] / total_stake;  // 666
        let exp_yield3 = total_yield * stakes[2] / total_stake;  // 333

        let final_bal1 = token.balance(&voucher1);
        let final_bal2 = token.balance(&voucher2);
        let final_bal3 = token.balance(&voucher3);

        assert_eq!(final_bal1, initial_bal1 + exp_yield1, "voucher1: stake + yield");
        assert_eq!(final_bal2, initial_bal2 + exp_yield2, "voucher2: stake + yield");
        assert_eq!(final_bal3, initial_bal3 + exp_yield3, "voucher3: stake + yield");

        // Vouches cleared
        assert!(s.client.get_vouches(&borrower).is_none());
    }
}
