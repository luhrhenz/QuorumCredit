/// Tests for withdraw_slash_treasury — admin-gated slash fund withdrawal.
#[cfg(test)]
mod withdraw_slash_treasury_tests {
    use crate::{ContractError, QuorumCreditContract, QuorumCreditContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Address, Env, Vec,
    };

    struct Setup {
        env: Env,
        client: QuorumCreditContractClient<'static>,
        admin: Address,
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

        env.ledger().with_mut(|l| l.timestamp = 120);

        Setup { env, client, admin, token_id: token_id.address() }
    }

    /// Seed the slash treasury by running a full slash cycle.
    fn seed_treasury(s: &Setup) -> i128 {
        let borrower = Address::generate(&s.env);
        let voucher_a = Address::generate(&s.env);
        let voucher_b = Address::generate(&s.env);

        StellarAssetClient::new(&s.env, &s.token_id).mint(&voucher_a, &600_000);
        StellarAssetClient::new(&s.env, &s.token_id).mint(&voucher_b, &400_000);
        s.client.vouch(&voucher_a, &borrower, &600_000, &s.token_id);
        s.client.vouch(&voucher_b, &borrower, &400_000, &s.token_id);
        s.client.request_loan(
            &borrower,
            &200_000,
            &800_000,
            &soroban_sdk::String::from_str(&s.env, "test"),
            &s.token_id,
        );
        // voucher_a holds 60% — quorum reached, slash fires
        s.client.vote_slash(&voucher_a, &borrower, &true);

        // 50% of 1_000_000 total stake = 500_000
        s.client.get_slash_treasury_balance()
    }

    /// Admin can withdraw the full treasury balance to a recipient.
    #[test]
    fn test_withdraw_slash_treasury_authorized() {
        let s = setup();
        let slashed = seed_treasury(&s);
        assert!(slashed > 0);

        let recipient = Address::generate(&s.env);
        let admins = Vec::from_array(&s.env, [s.admin.clone()]);

        s.client.withdraw_slash_treasury(&admins, &recipient, &slashed);

        // Treasury should be empty
        assert_eq!(s.client.get_slash_treasury_balance(), 0);

        // Recipient should hold the withdrawn amount
        let token = soroban_sdk::token::Client::new(&s.env, &s.token_id);
        assert_eq!(token.balance(&recipient), slashed);
    }

    /// Admin can do a partial withdrawal, leaving the remainder in the treasury.
    #[test]
    fn test_withdraw_slash_treasury_partial() {
        let s = setup();
        let slashed = seed_treasury(&s);

        let recipient = Address::generate(&s.env);
        let admins = Vec::from_array(&s.env, [s.admin.clone()]);
        let withdraw_amount = slashed / 2;

        s.client.withdraw_slash_treasury(&admins, &recipient, &withdraw_amount);

        assert_eq!(s.client.get_slash_treasury_balance(), slashed - withdraw_amount);
        let token = soroban_sdk::token::Client::new(&s.env, &s.token_id);
        assert_eq!(token.balance(&recipient), withdraw_amount);
    }

    /// Withdrawing more than the treasury balance must panic.
    #[test]
    fn test_withdraw_slash_treasury_exceeds_balance() {
        let s = setup();
        let slashed = seed_treasury(&s);

        let recipient = Address::generate(&s.env);
        let admins = Vec::from_array(&s.env, [s.admin.clone()]);

        let result = s.client.try_withdraw_slash_treasury(&admins, &recipient, &(slashed + 1));
        assert!(result.is_err(), "withdrawal exceeding balance must fail");
    }

    /// Withdrawing zero must panic.
    #[test]
    fn test_withdraw_slash_treasury_zero_amount() {
        let s = setup();
        seed_treasury(&s);

        let recipient = Address::generate(&s.env);
        let admins = Vec::from_array(&s.env, [s.admin.clone()]);

        let result = s.client.try_withdraw_slash_treasury(&admins, &recipient, &0);
        assert!(result.is_err(), "zero amount withdrawal must fail");
    }

    /// A non-admin signer must be rejected.
    #[test]
    fn test_withdraw_slash_treasury_unauthorized() {
        let s = setup();
        seed_treasury(&s);

        let recipient = Address::generate(&s.env);
        let outsider = Address::generate(&s.env);
        let fake_admins = Vec::from_array(&s.env, [outsider]);

        let result = s.client.try_withdraw_slash_treasury(&fake_admins, &recipient, &100_000);
        assert!(result.is_err(), "non-admin must be rejected");
    }
}
