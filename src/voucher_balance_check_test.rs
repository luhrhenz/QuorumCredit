/// Voucher Balance Verification Tests
///
/// Verifies that vouch checks voucher balance before transfer and returns
/// a structured error if insufficient, instead of panicking.
#[cfg(test)]
mod voucher_balance_check_tests {
    use crate::{ContractError, QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::Address as _,
        token::StellarAssetClient,
        Address, Env, Vec,
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

        StellarAssetClient::new(&env, &token_id.address()).mint(&contract_id, &10_000_000);

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &1, &token_id.address());

        Setup {
            env,
            client,
            token_id: token_id.address(),
        }
    }

    /// Verify that vouch returns InsufficientVoucherBalance error when voucher
    /// doesn't have enough tokens, instead of panicking during transfer.
    #[test]
    fn test_vouch_insufficient_balance_returns_error() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);

        // Mint only 500 tokens to voucher
        StellarAssetClient::new(&s.env, &s.token_id).mint(&voucher, &500);

        // Attempt to vouch with 1000 tokens (more than balance)
        let result = s.client.try_vouch(&voucher, &borrower, &1_000, &s.token_id);
        
        // Should return InsufficientVoucherBalance error
        assert_eq!(result, Err(Ok(ContractError::InsufficientVoucherBalance)));
    }

    /// Verify that vouch succeeds when voucher has sufficient balance.
    #[test]
    fn test_vouch_succeeds_with_sufficient_balance() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);

        // Mint exactly 1000 tokens to voucher
        StellarAssetClient::new(&s.env, &s.token_id).mint(&voucher, &1_000);

        // Vouch with 1000 tokens should succeed
        let result = s.client.try_vouch(&voucher, &borrower, &1_000, &s.token_id);
        assert!(result.is_ok(), "vouch should succeed with sufficient balance");

        // Verify vouch was recorded
        assert!(s.client.vouch_exists(&voucher, &borrower));
    }

    /// Verify that vouch fails when voucher has zero balance.
    #[test]
    fn test_vouch_zero_balance_returns_error() {
        let s = setup();
        let voucher = Address::generate(&s.env);
        let borrower = Address::generate(&s.env);

        // Don't mint any tokens to voucher (balance = 0)

        // Attempt to vouch with any amount
        let result = s.client.try_vouch(&voucher, &borrower, &100, &s.token_id);
        
        // Should return InsufficientVoucherBalance error
        assert_eq!(result, Err(Ok(ContractError::InsufficientVoucherBalance)));
    }
}
