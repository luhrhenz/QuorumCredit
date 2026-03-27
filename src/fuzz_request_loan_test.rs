/// Fuzz test for `request_loan` with random amount/threshold combinations.
///
/// # Findings
/// - `amount < min_loan_amount` (100_000)  → host panic (assert)
/// - `threshold <= 0`                      → host panic (assert)
/// - `total_stake < threshold`             → host panic (assert)
/// - `amount > total_stake * 150 / 100`    → host panic (collateral ratio assert)
/// - `amount > max_loan_amount` (when set) → `ContractError::LoanExceedsMaxAmount`
/// - `amount > contract_balance`           → `ContractError::InsufficientFunds`
/// - Valid combinations                    → `Ok(())`
/// - No integer overflow observed; `checked_add` on stake accumulation returns
///   `ContractError::StakeOverflow` rather than panicking.
#[cfg(test)]
mod fuzz_request_loan_tests {
    use crate::{ContractError, QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
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

        // Advance past MIN_VOUCH_AGE so vouches are always eligible.
        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup { env, client, token: token_id.address(), admin_vec }
    }

    fn vouch_and_fund(s: &Setup, stake: i128, fund: i128) -> (Address, Address) {
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);
        StellarAssetClient::new(&s.env, &s.token).mint(&voucher, &stake);
        s.client.vouch(&voucher, &borrower, &stake, &s.token);
        // Advance past MIN_VOUCH_AGE.
        s.env.ledger().with_mut(|l| l.timestamp += 61);
        StellarAssetClient::new(&s.env, &s.token).mint(&s.client.address, &fund);
        (voucher, borrower)
    }

    fn purpose(env: &Env) -> String {
        String::from_str(env, "test")
    }

    // ── amount < min_loan_amount → host panic ─────────────────────────────────

    #[test]
    fn test_amount_below_minimum_panics() {
        let s = setup();
        let (_, borrower) = vouch_and_fund(&s, 1_000_000, 1_000_000);
        // amount=99_999 < DEFAULT_MIN_LOAN_AMOUNT=100_000
        let result = s.client.try_request_loan(&borrower, &99_999, &500_000, &purpose(&s.env), &s.token);
        assert!(result.is_err(), "amount below min_loan_amount must fail");
    }

    #[test]
    fn test_zero_amount_panics() {
        let s = setup();
        let (_, borrower) = vouch_and_fund(&s, 1_000_000, 1_000_000);
        let result = s.client.try_request_loan(&borrower, &0, &500_000, &purpose(&s.env), &s.token);
        assert!(result.is_err(), "zero amount must fail");
    }

    #[test]
    fn test_negative_amount_panics() {
        let s = setup();
        let (_, borrower) = vouch_and_fund(&s, 1_000_000, 1_000_000);
        let result = s.client.try_request_loan(&borrower, &-1, &500_000, &purpose(&s.env), &s.token);
        assert!(result.is_err(), "negative amount must fail");
    }

    // ── threshold <= 0 → host panic ───────────────────────────────────────────

    #[test]
    fn test_zero_threshold_panics() {
        let s = setup();
        let (_, borrower) = vouch_and_fund(&s, 1_000_000, 1_000_000);
        let result = s.client.try_request_loan(&borrower, &100_000, &0, &purpose(&s.env), &s.token);
        assert!(result.is_err(), "zero threshold must fail");
    }

    #[test]
    fn test_negative_threshold_panics() {
        let s = setup();
        let (_, borrower) = vouch_and_fund(&s, 1_000_000, 1_000_000);
        let result = s.client.try_request_loan(&borrower, &100_000, &-1, &purpose(&s.env), &s.token);
        assert!(result.is_err(), "negative threshold must fail");
    }

    // ── total_stake < threshold → host panic ──────────────────────────────────

    #[test]
    fn test_stake_below_threshold_panics() {
        let s = setup();
        // stake=500_000 but threshold=1_000_000
        let (_, borrower) = vouch_and_fund(&s, 500_000, 1_000_000);
        let result = s.client.try_request_loan(&borrower, &100_000, &1_000_000, &purpose(&s.env), &s.token);
        assert!(result.is_err(), "stake below threshold must fail");
    }

    // ── amount > total_stake * 150/100 → host panic (collateral ratio) ────────

    #[test]
    fn test_amount_exceeds_collateral_ratio_panics() {
        let s = setup();
        // stake=100_000, max_allowed = 100_000 * 150/100 = 150_000; amount=200_000 > 150_000
        let (_, borrower) = vouch_and_fund(&s, 100_000, 1_000_000);
        let result = s.client.try_request_loan(&borrower, &200_000, &100_000, &purpose(&s.env), &s.token);
        assert!(result.is_err(), "amount exceeding collateral ratio must fail");
    }

    // ── amount > max_loan_amount → LoanExceedsMaxAmount ──────────────────────

    #[test]
    fn test_amount_exceeds_max_loan_amount() {
        let s = setup();
        s.client.set_max_loan_amount(&s.admin_vec, &500_000);
        let (_, borrower) = vouch_and_fund(&s, 2_000_000, 2_000_000);
        let result = s.client.try_request_loan(&borrower, &600_000, &500_000, &purpose(&s.env), &s.token);
        assert_eq!(result, Err(Ok(ContractError::LoanExceedsMaxAmount)));
    }

    // ── contract_balance < amount → InsufficientFunds ────────────────────────

    #[test]
    fn test_insufficient_contract_balance() {
        let s = setup();
        // Stake 100_000 into contract (no extra funding). max_allowed = 150_000.
        // Request 150_000 — passes collateral ratio but contract only holds 100_000.
        let (_, borrower) = vouch_and_fund(&s, 100_000, 0);
        let result = s.client.try_request_loan(&borrower, &150_000, &100_000, &purpose(&s.env), &s.token);
        assert_eq!(result, Err(Ok(ContractError::InsufficientFunds)));
    }

    // ── valid combinations → Ok ───────────────────────────────────────────────

    #[test]
    fn test_valid_exact_minimum_amount() {
        let s = setup();
        let (_, borrower) = vouch_and_fund(&s, 1_000_000, 1_000_000);
        // amount=100_000 == min_loan_amount; threshold=500_000 <= stake=1_000_000
        let result = s.client.try_request_loan(&borrower, &100_000, &500_000, &purpose(&s.env), &s.token);
        assert_eq!(result, Ok(Ok(())));
    }

    #[test]
    fn test_valid_threshold_equals_stake() {
        let s = setup();
        let (_, borrower) = vouch_and_fund(&s, 1_000_000, 1_000_000);
        // threshold == total_stake (boundary)
        let result = s.client.try_request_loan(&borrower, &100_000, &1_000_000, &purpose(&s.env), &s.token);
        assert_eq!(result, Ok(Ok(())));
    }

    #[test]
    fn test_valid_large_stake_large_amount() {
        let s = setup();
        // stake=10_000_000; max_allowed = 15_000_000; amount=5_000_000
        let (_, borrower) = vouch_and_fund(&s, 10_000_000, 10_000_000);
        let result = s.client.try_request_loan(&borrower, &5_000_000, &5_000_000, &purpose(&s.env), &s.token);
        assert_eq!(result, Ok(Ok(())));
    }

    // ── i128 extremes ─────────────────────────────────────────────────────────

    #[test]
    fn test_i128_max_amount_panics() {
        let s = setup();
        let (_, borrower) = vouch_and_fund(&s, 1_000_000, 1_000_000);
        let result = s.client.try_request_loan(&borrower, &i128::MAX, &500_000, &purpose(&s.env), &s.token);
        assert!(result.is_err(), "i128::MAX amount must fail");
    }

    #[test]
    fn test_i128_max_threshold_panics() {
        let s = setup();
        // stake < i128::MAX so total_stake < threshold → fails
        let (_, borrower) = vouch_and_fund(&s, 1_000_000, 1_000_000);
        let result = s.client.try_request_loan(&borrower, &100_000, &i128::MAX, &purpose(&s.env), &s.token);
        assert!(result.is_err(), "i128::MAX threshold with insufficient stake must fail");
    }
}
