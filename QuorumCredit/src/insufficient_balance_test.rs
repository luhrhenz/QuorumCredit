/// Test: repay fails when the contract has insufficient balance to cover yield payouts.
///
/// Scenario:
///   - Contract is funded with exactly the loan amount (no yield reserve).
///   - After disbursement the contract balance is 0.
///   - Borrower repays principal + yield; contract receives the payment but then
///     must transfer `stake + yield` back to the voucher — which exceeds what it holds.
///   - The token transfer panics at the host level; `try_repay` must return an error.
#[cfg(test)]
mod insufficient_balance_tests {
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
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin.clone()]);
        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let contract_id = env.register_contract(None, QuorumCreditContract);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &1, &token_id.address());

        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup { env, client, token: token_id.address() }
    }

    /// repay must fail when the contract balance cannot cover stake + yield payout to vouchers.
    #[test]
    fn test_repay_fails_when_contract_has_insufficient_balance_for_yield() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);

        let stake: i128 = 1_000_000;
        let loan_amount: i128 = 500_000;
        // yield = 2% of loan_amount = 10_000; total_owed = 510_000
        // payout required = stake + yield = 1_010_000

        // Fund voucher and vouch — stake is transferred into the contract.
        StellarAssetClient::new(&s.env, &s.token).mint(&voucher, &stake);
        s.client.vouch(&voucher, &borrower, &stake, &s.token);

        // Advance time past MIN_VOUCH_AGE (60s) so the vouch is eligible.
        s.env.ledger().with_mut(|l| l.timestamp += 61);

        // Fund contract with the loan amount.
        StellarAssetClient::new(&s.env, &s.token).mint(&s.client.address, &loan_amount);

        s.client.request_loan(
            &borrower,
            &loan_amount,
            &stake,
            &String::from_str(&s.env, "test"),
            &s.token,
        );

        // Contract now holds the stake (1_000_000). Drain it via burn so the contract
        // cannot cover the stake + yield payout when repay is called.
        // mock_all_auths() allows burning on behalf of the contract address.
        let contract_balance = TokenClient::new(&s.env, &s.token).balance(&s.client.address);
        TokenClient::new(&s.env, &s.token).burn(&s.client.address, &(contract_balance - 1));

        assert_eq!(
            TokenClient::new(&s.env, &s.token).balance(&s.client.address),
            1,
            "contract should have only 1 stroop after drain"
        );

        // Borrower repays principal + yield.
        let yield_amount: i128 = loan_amount * 200 / 10_000; // 2% = 10_000
        let total_owed = loan_amount + yield_amount;
        StellarAssetClient::new(&s.env, &s.token).mint(&borrower, &total_owed);

        // repay must fail: after receiving total_owed (510_000), contract has 510_001
        // but must pay out stake + yield (1_000_000 + 10_000 = 1_010_000) — insufficient.
        let result = s.client.try_repay(&borrower, &total_owed);
        assert!(
            result.is_err(),
            "repay must fail when contract cannot cover stake + yield payout"
        );
    }
}
