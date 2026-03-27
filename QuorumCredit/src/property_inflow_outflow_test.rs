/// Property test: total outflow never exceeds total inflow.
///
/// The invariant tracked after every operation:
///   `contract_balance == total_inflow - total_outflow`
///
/// Inflows:  vouch (stake transfer in), repayment (borrower pays principal+yield),
///           any direct mint to contract.
/// Outflows: loan disbursement, stake+yield returned to vouchers on repay,
///           unslashed stake returned to vouchers on slash.
///           (Slashed portion stays in contract as SlashTreasury — not an outflow.)
///
/// # Scenarios
/// 1. Vouch only
/// 2. Vouch → request_loan
/// 3. Vouch → request_loan → repay
/// 4. Vouch → request_loan → slash (via vote_slash with quorum=1)
/// 5. Multiple vouchers, multiple loans, mixed repay cycles
#[cfg(test)]
mod property_inflow_outflow_tests {
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
        admin_vec: Vec<Address>,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admin_vec = Vec::from_array(&env, [admin.clone()]);
        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let contract_id = env.register_contract(None, QuorumCreditContract);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admin_vec, &1, &token_id.address());
        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup { env, client, token: token_id.address(), admin_vec }
    }

    fn balance(s: &Setup, addr: &Address) -> i128 {
        TokenClient::new(&s.env, &s.token).balance(addr)
    }

    fn mint(s: &Setup, to: &Address, amount: i128) {
        StellarAssetClient::new(&s.env, &s.token).mint(to, &amount);
    }

    fn purpose(env: &Env) -> String {
        String::from_str(env, "test")
    }

    fn assert_invariant(s: &Setup, inflow: i128, outflow: i128, ctx: &str) {
        let bal = balance(s, &s.client.address);
        assert_eq!(
            bal,
            inflow - outflow,
            "{ctx}: balance={bal} != inflow({inflow}) - outflow({outflow})"
        );
        assert!(outflow <= inflow, "{ctx}: outflow({outflow}) exceeded inflow({inflow})");
    }

    /// Scenario 1: vouch only — stake is inflow, nothing leaves.
    #[test]
    fn test_vouch_only() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let stake = 1_000_000i128;

        mint(&s, &voucher, stake);
        s.client.vouch(&voucher, &borrower, &stake, &s.token);

        assert_invariant(&s, stake, 0, "after vouch");
    }

    /// Scenario 2: vouch → request_loan — disbursement is outflow.
    #[test]
    fn test_vouch_then_loan() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let stake = 1_000_000i128;
        let loan = 500_000i128;

        mint(&s, &voucher, stake);
        mint(&s, &s.client.address, loan);
        let mut inflow = stake + loan;
        let mut outflow = 0i128;

        s.client.vouch(&voucher, &borrower, &stake, &s.token);
        assert_invariant(&s, inflow, outflow, "after vouch");

        s.env.ledger().with_mut(|l| l.timestamp += 61);
        s.client.request_loan(&borrower, &loan, &stake, &purpose(&s.env), &s.token);
        outflow += loan;
        assert_invariant(&s, inflow, outflow, "after request_loan");
    }

    /// Scenario 3: full repay cycle — repayment in, stake+yield out.
    #[test]
    fn test_full_repay_cycle() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let stake = 1_000_000i128;
        let loan = 500_000i128;
        let yield_amt = loan * 200 / 10_000; // 10_000
        let total_owed = loan + yield_amt;

        mint(&s, &voucher, stake);
        mint(&s, &s.client.address, loan);
        let mut inflow = stake + loan;
        let mut outflow = 0i128;

        s.client.vouch(&voucher, &borrower, &stake, &s.token);
        s.env.ledger().with_mut(|l| l.timestamp += 61);
        s.client.request_loan(&borrower, &loan, &stake, &purpose(&s.env), &s.token);
        outflow += loan;
        assert_invariant(&s, inflow, outflow, "after request_loan");

        mint(&s, &borrower, total_owed);
        s.client.repay(&borrower, &total_owed);
        inflow += total_owed;
        outflow += stake + yield_amt; // stake returned + yield paid
        assert_invariant(&s, inflow, outflow, "after repay");
    }

    /// Scenario 4: slash cycle — unslashed remainder returned to voucher (outflow);
    /// slashed portion stays in contract as SlashTreasury (not an outflow).
    #[test]
    fn test_slash_cycle() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        let stake = 1_000_000i128;
        let loan = 500_000i128;
        let slash_bps = 5_000i128; // default 50%
        let slashed = stake * slash_bps / 10_000; // 500_000 burned (stays in contract)
        let returned = stake - slashed;            // 500_000 returned to voucher (outflow)

        mint(&s, &voucher, stake);
        mint(&s, &s.client.address, loan);
        let mut inflow = stake + loan;
        let mut outflow = 0i128;

        s.client.vouch(&voucher, &borrower, &stake, &s.token);
        s.env.ledger().with_mut(|l| l.timestamp += 61);
        s.client.request_loan(&borrower, &loan, &stake, &purpose(&s.env), &s.token);
        outflow += loan;
        assert_invariant(&s, inflow, outflow, "after request_loan");

        // Set quorum to 1 bps so a single voucher vote triggers slash immediately.
        s.client.set_slash_vote_quorum(&s.admin_vec, &1);
        s.client.vote_slash(&voucher, &borrower, &true);
        outflow += returned; // only the unslashed remainder leaves the contract
        assert_invariant(&s, inflow, outflow, "after slash");

        // Slashed amount is tracked in SlashTreasury (still in contract balance).
        assert_eq!(
            s.client.get_slash_treasury_balance(),
            slashed,
            "slashed amount must remain in contract as SlashTreasury"
        );
    }

    /// Scenario 5: multiple independent vouch→loan→repay cycles.
    #[test]
    fn test_multiple_cycles_invariant_holds() {
        let s = setup();
        let fund = 3_000_000i128;
        mint(&s, &s.client.address, fund);
        let mut inflow = fund;
        let mut outflow = 0i128;

        for _ in 0..3 {
            let voucher = Address::generate(&s.env);
            let borrower = Address::generate(&s.env);
            let stake = 1_000_000i128;
            let loan = 500_000i128;
            let yield_amt = loan * 200 / 10_000;
            let total_owed = loan + yield_amt;

            mint(&s, &voucher, stake);
            s.client.vouch(&voucher, &borrower, &stake, &s.token);
            inflow += stake;
            assert_invariant(&s, inflow, outflow, "after vouch");

            s.env.ledger().with_mut(|l| l.timestamp += 61);
            s.client.request_loan(&borrower, &loan, &stake, &purpose(&s.env), &s.token);
            outflow += loan;
            assert_invariant(&s, inflow, outflow, "after request_loan");

            mint(&s, &borrower, total_owed);
            s.client.repay(&borrower, &total_owed);
            inflow += total_owed;
            outflow += stake + yield_amt;
            assert_invariant(&s, inflow, outflow, "after repay");
        }
    }
}
