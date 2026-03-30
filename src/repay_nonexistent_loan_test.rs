/// Repay on Non-Existent Loan Tests
///
/// Verifies that repay panics when no loan record exists for the borrower.
#[cfg(test)]
mod repay_nonexistent_loan_tests {
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

    /// Verify that try_repay returns NoActiveLoan error when no loan exists.
    #[test]
    fn test_repay_returns_error_when_no_loan_exists() {
        let s = setup();
        let borrower = Address::generate(&s.env);

        // try_repay should return Err(NoActiveLoan) instead of panicking.
        let result = s.client.try_repay(&borrower, &100_000);
        assert_eq!(result, Err(Ok(ContractError::NoActiveLoan)));
    }
}
