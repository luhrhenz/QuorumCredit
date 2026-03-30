/// Tests verifying that yield_bps and slash_bps are read from the Config
/// struct in instance storage — not from compile-time constants.
///
/// Covers issue #169: Move YIELD_BPS / SLASH_BPS constants to Config struct.
#[cfg(test)]
mod config_bps_tests {
    use crate::{Config, QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Address, Env, String, Vec,
    };

    struct Setup {
        env: Env,
        client: QuorumCreditContractClient<'static>,
        admin: Address,
        token: Address,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);
        let token = env.register_stellar_asset_contract_v2(admin.clone());
        let contract_id = env.register_contract(None, QuorumCreditContract);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &1, &token.address());

        // Fund contract so loans can be disbursed and yield paid out.
        StellarAssetClient::new(&env, &token.address()).mint(&contract_id, &100_000_000);

        // Advance past MIN_VOUCH_AGE.
        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup { env, client, admin, token: token.address() }
    }

    fn do_vouch(s: &Setup, voucher: &Address, borrower: &Address, stake: i128) {
        StellarAssetClient::new(&s.env, &s.token).mint(voucher, &stake);
        s.client.vouch(voucher, borrower, &stake, &s.token);
    }

    fn do_loan(s: &Setup, borrower: &Address, amount: i128, threshold: i128) {
        s.client.request_loan(
            borrower,
            &amount,
            &threshold,
            &String::from_str(&s.env, "test"),
            &s.token,
        );
    }

    fn token_balance(s: &Setup, addr: &Address) -> i128 {
        soroban_sdk::token::Client::new(&s.env, &s.token).balance(addr)
    }

    fn set_bps(s: &Setup, yield_bps: i128, slash_bps: i128) {
        let mut cfg = s.client.get_config();
        cfg.yield_bps = yield_bps;
        cfg.slash_bps = slash_bps;
        let admin_signers = Vec::from_array(&s.env, [s.admin.clone()]);
        s.client.set_config(&admin_signers, &cfg);
    }

    /// Voucher earns yield at the rate stored in Config.yield_bps, not a
    /// hardcoded constant. Here we set yield_bps = 1000 (10%) and verify the
    /// voucher receives stake + 10% after repayment.
    #[test]
    fn test_yield_bps_read_from_config() {
        let s = setup();
        set_bps(&s, 1_000, 5_000); // 10% yield, 50% slash

        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let stake: i128 = 1_000_000;
        let loan_amount: i128 = 100_000;

        do_vouch(&s, &voucher, &borrower, stake);
        do_loan(&s, &borrower, loan_amount, stake);

        // Borrower repays principal + 10% yield.
        let expected_yield = loan_amount * 1_000 / 10_000; // 10_000
        let total_owed = loan_amount + expected_yield;
        StellarAssetClient::new(&s.env, &s.token).mint(&borrower, &total_owed);
        s.client.repay(&borrower, &total_owed);

        // Voucher should have received stake + yield.
        let voucher_balance = token_balance(&s, &voucher);
        assert_eq!(
            voucher_balance,
            stake + expected_yield,
            "voucher balance should equal stake + 10% yield from config"
        );
    }

    /// Voucher loses the fraction of stake defined by Config.slash_bps, not a
    /// hardcoded constant. Here we set slash_bps = 2000 (20%) and verify only
    /// 20% is burned on default.
    #[test]
    fn test_slash_bps_read_from_config() {
        let s = setup();
        set_bps(&s, 200, 2_000); // default yield, 20% slash

        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let stake: i128 = 1_000_000;

        do_vouch(&s, &voucher, &borrower, stake);
        do_loan(&s, &borrower, 100_000, stake);

        let admin_signers = Vec::from_array(&s.env, [s.admin.clone()]);
        s.client.slash(&admin_signers, &borrower);

        // Voucher should receive stake * (1 - 20%) = 800_000.
        let expected_returned = stake * (10_000 - 2_000) / 10_000;
        let voucher_balance = token_balance(&s, &voucher);
        assert_eq!(
            voucher_balance,
            expected_returned,
            "voucher should lose only the slash_bps fraction defined in config"
        );
    }

    /// Changing yield_bps via set_config takes effect on the next loan
    /// disbursement — the new rate is locked in at request_loan time.
    #[test]
    fn test_config_yield_bps_applied_on_next_loan() {
        let s = setup();

        // First loan at default 2% yield.
        let v1 = Address::generate(&s.env);
        let b1 = Address::generate(&s.env);
        do_vouch(&s, &v1, &b1, 1_000_000);
        do_loan(&s, &b1, 100_000, 1_000_000);
        let loan1 = s.client.get_loan(&b1).unwrap();
        assert_eq!(loan1.total_yield, 2_000, "default yield_bps=200 → 2% of 100_000 = 2_000");

        // Update yield_bps to 500 (5%).
        set_bps(&s, 500, 5_000);

        // Second loan should lock in 5% yield.
        let v2 = Address::generate(&s.env);
        let b2 = Address::generate(&s.env);
        do_vouch(&s, &v2, &b2, 1_000_000);
        do_loan(&s, &b2, 100_000, 1_000_000);
        let loan2 = s.client.get_loan(&b2).unwrap();
        assert_eq!(loan2.total_yield, 5_000, "updated yield_bps=500 → 5% of 100_000 = 5_000");
    }
}
