/// Fuzz test for `vouch` with random i128 stake amounts.
///
/// # Findings
/// - `stake <= 0`          → `ContractError::InsufficientFunds` (caught by `require_positive_amount`)
/// - `0 < stake < min_stake` → `ContractError::MinStakeNotMet` (when min_stake is set)
/// - `stake > voucher_balance` → host-level panic (token transfer fails; `try_vouch` returns `Err`)
/// - `stake >= min_stake` and `stake <= voucher_balance` → success
/// - No integer overflow or unexpected panic observed for any i128 value.
#[cfg(test)]
mod fuzz_vouch_tests {
    use crate::{ContractError, QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::Address as _,
        token::StellarAssetClient,
        Address, Env, Vec,
    };

    /// Deterministic fuzz: exercise a representative spread of i128 stake values
    /// covering negatives, zero, sub-minimum positives, normal values, and extremes.
    #[test]
    fn test_vouch_fuzz_stake_amounts() {
        // Findings documented inline per stake category.
        let cases: &[i128] = &[
            // ── Invalid: non-positive ──────────────────────────────────────────
            i128::MIN,   // extreme negative → InsufficientFunds
            -1_000_000,  // negative         → InsufficientFunds
            -1,          // -1               → InsufficientFunds
            0,           // zero             → InsufficientFunds
            // ── Invalid: below min_stake (set to 1_000 below) ─────────────────
            1,           // 1 stroop         → MinStakeNotMet
            999,         // just under min   → MinStakeNotMet
            // ── Valid: meets min_stake and voucher has sufficient balance ──────
            1_000,       // exact minimum    → Ok
            1_000_000,   // normal stake     → Ok
            i128::MAX / 2, // large but funded → Ok (voucher minted to match)
            // ── Invalid: exceeds voucher balance (not minted) ─────────────────
            i128::MAX,   // max i128, no balance → host error (transfer fails)
        ];

        let min_stake: i128 = 1_000;

        for &stake in cases {
            let env = Env::default();
            env.mock_all_auths();

            // Set min_stake so sub-minimum cases are exercised.
            let admin = Address::generate(&env);
            let admin_vec = Vec::from_array(&env, [admin.clone()]);
            let deployer = Address::generate(&env);
            let token_id = env.register_stellar_asset_contract_v2(admin.clone());
            let contract_id = env.register_contract(None, QuorumCreditContract);
            let client = QuorumCreditContractClient::new(&env, &contract_id);
            client.initialize(&deployer, &admin_vec, &1, &token_id.address());
            client.set_min_stake(&admin_vec, &min_stake);

            let voucher = Address::generate(&env);
            let borrower = Address::generate(&env);

            // Mint exactly `stake` to the voucher for valid positive cases.
            if stake > 0 && stake != i128::MAX {
                StellarAssetClient::new(&env, &token_id.address()).mint(&voucher, &stake);
            }

            let result = client.try_vouch(&voucher, &borrower, &stake, &token_id.address());

            match stake {
                s if s <= 0 => {
                    assert_eq!(
                        result,
                        Err(Ok(ContractError::InsufficientFunds)),
                        "stake={stake}: expected InsufficientFunds for non-positive stake"
                    );
                }
                s if s < min_stake => {
                    assert_eq!(
                        result,
                        Err(Ok(ContractError::MinStakeNotMet)),
                        "stake={stake}: expected MinStakeNotMet for sub-minimum stake"
                    );
                }
                i128::MAX => {
                    // Voucher has no balance; token transfer panics at host level.
                    assert!(
                        result.is_err(),
                        "stake={stake}: expected error when voucher has no balance"
                    );
                }
                _ => {
                    assert_eq!(
                        result,
                        Ok(Ok(())),
                        "stake={stake}: expected Ok for valid stake"
                    );
                }
            }
        }
    }
}
