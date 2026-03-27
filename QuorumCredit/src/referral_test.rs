#[cfg(test)]
mod referral_tests {
    use crate::{ContractError, QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{StellarAssetClient, TokenClient},
        Address, Env, String, Vec,
    };

    struct Setup {
        env: Env,
        client: QuorumCreditContractClient<'static>,
        contract_id: Address,
        token: Address,
        admin: Address,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);
        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let contract_id = env.register_contract(None, QuorumCreditContract);

        // Fund contract for loans + yield + referral bonuses.
        StellarAssetClient::new(&env, &token_id.address()).mint(&contract_id, &50_000_000);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &1, &token_id.address());

        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup { env, client, contract_id, token: token_id.address(), admin }
    }

    fn do_vouch(s: &Setup, voucher: &Address, borrower: &Address, stake: i128) {
        StellarAssetClient::new(&s.env, &s.token).mint(voucher, &stake);
        s.client.vouch(voucher, borrower, &stake, &s.token);
    }

    fn do_loan(s: &Setup, borrower: &Address, amount: i128) {
        s.client.request_loan(
            borrower,
            &amount,
            &500_000,
            &String::from_str(&s.env, "test"),
            &s.token,
        );
    }

    /// Referrer receives 1% bonus on full repayment.
    #[test]
    fn test_referral_bonus_paid_on_repayment() {
        let s = setup();
        let referrer = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        do_vouch(&s, &voucher, &borrower, 1_000_000);
        s.client.register_referral(&borrower, &referrer);
        do_loan(&s, &borrower, 100_000);

        // Borrower repays principal + yield (2% of 100_000 = 2_000).
        StellarAssetClient::new(&s.env, &s.token).mint(&borrower, &102_000);
        s.client.repay(&borrower, &102_000);

        // Referral bonus = 1% of 100_000 = 1_000.
        let referrer_balance = TokenClient::new(&s.env, &s.token)
            .balance(&referrer);
        assert_eq!(referrer_balance, 1_000);
    }

    /// No referral registered → no bonus, repayment succeeds normally.
    #[test]
    fn test_no_referral_no_bonus() {
        let s = setup();
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        do_vouch(&s, &voucher, &borrower, 1_000_000);
        do_loan(&s, &borrower, 100_000);

        StellarAssetClient::new(&s.env, &s.token).mint(&borrower, &102_000);
        s.client.repay(&borrower, &102_000);

        assert_eq!(s.client.loan_status(&borrower), crate::LoanStatus::Repaid);
    }

    /// Borrower cannot refer themselves.
    #[test]
    fn test_self_referral_rejected() {
        let s = setup();
        let borrower = Address::generate(&s.env);

        let result = s.client.try_register_referral(&borrower, &borrower);
        assert!(result.is_err());
    }

    /// get_referrer returns the registered referrer.
    #[test]
    fn test_get_referrer_returns_correct_address() {
        let s = setup();
        let referrer = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);

        assert!(s.client.get_referrer(&borrower).is_none());
        s.client.register_referral(&borrower, &referrer);
        assert_eq!(s.client.get_referrer(&borrower), Some(referrer));
    }

    /// Admin can change the bonus rate; new rate is applied on next repayment.
    #[test]
    fn test_custom_referral_bonus_bps() {
        let s = setup();
        let admins = Vec::from_array(&s.env, [s.admin.clone()]);
        // Set bonus to 2%.
        s.client.set_referral_bonus_bps(&admins, &200);
        assert_eq!(s.client.get_referral_bonus_bps(), 200);

        let referrer = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let voucher = Address::generate(&s.env);

        do_vouch(&s, &voucher, &borrower, 1_000_000);
        s.client.register_referral(&borrower, &referrer);
        do_loan(&s, &borrower, 100_000);

        StellarAssetClient::new(&s.env, &s.token).mint(&borrower, &102_000);
        s.client.repay(&borrower, &102_000);

        // 2% of 100_000 = 2_000.
        let referrer_balance = TokenClient::new(&s.env, &s.token)
            .balance(&referrer);
        assert_eq!(referrer_balance, 2_000);
    }
}
