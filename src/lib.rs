#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, token, Address, Env, Vec};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Yield paid to vouchers on repayment: 2% of their stake.
const YIELD_BPS: i128 = 200;
/// Slash penalty on default: 50% of voucher stake burned.
const SLASH_BPS: i128 = 5000;

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    InsufficientFunds = 1,
}

// ── Storage Keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Loan(Address),    // borrower → LoanRecord
    Vouches(Address), // borrower → Vec<VouchRecord>
    Admin,            // Address allowed to call slash
    Token,            // XLM token contract address
}

// ── Data Types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct LoanRecord {
    pub borrower: Address,
    pub amount: i128, // in stroops
    pub repaid: bool,
    pub defaulted: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct VouchRecord {
    pub voucher: Address,
    pub stake: i128, // in stroops
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct QuorumCreditContract;

#[contractimpl]
impl QuorumCreditContract {
    /// One-time initialisation: set admin and XLM token address.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        assert!(
            !env.storage().instance().has(&DataKey::Admin),
            "already initialized"
        );
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
    }

    /// Stake XLM to vouch for a borrower.
    pub fn vouch(env: Env, voucher: Address, borrower: Address, stake: i128) {
        voucher.require_auth();

        // Transfer stake from voucher into the contract.
        let token = Self::token(&env);
        token.transfer(&voucher, &env.current_contract_address(), &stake);

        let mut vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower.clone()))
            .unwrap_or(Vec::new(&env));

        vouches.push_back(VouchRecord { voucher, stake });
        env.storage()
            .persistent()
            .set(&DataKey::Vouches(borrower), &vouches);
    }

    /// Disburse a microloan if total vouched stake meets the threshold.
    pub fn request_loan(
        env: Env,
        borrower: Address,
        amount: i128,
        threshold: i128,
    ) -> Result<(), ContractError> {
        borrower.require_auth();

        let vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower.clone()))
            .unwrap_or(Vec::new(&env));

        let total_stake: i128 = vouches.iter().map(|v| v.stake).sum();
        assert!(total_stake >= threshold, "insufficient trust stake");

        // Verify the contract holds enough XLM to cover the loan.
        let token = Self::token(&env);
        let contract_balance = token.balance(&env.current_contract_address());
        if contract_balance < amount {
            return Err(ContractError::InsufficientFunds);
        }

        // Send loan amount to borrower.
        token.transfer(&env.current_contract_address(), &borrower, &amount);

        env.storage().persistent().set(
            &DataKey::Loan(borrower.clone()),
            &LoanRecord {
                borrower,
                amount,
                repaid: false,
                defaulted: false,
            },
        );
        Ok(())
    }

    /// Borrower repays loan; vouchers receive 2% yield on their stake.
    pub fn repay(env: Env, borrower: Address) {
        borrower.require_auth();

        let mut loan: LoanRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Loan(borrower.clone()))
            .expect("no active loan");

        assert!(!loan.defaulted, "loan already defaulted");
        assert!(!loan.repaid, "loan already repaid");

        // Collect repayment from borrower.
        let token = Self::token(&env);
        token.transfer(&borrower, &env.current_contract_address(), &loan.amount);

        // Return stake + 2% yield to each voucher.
        let vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower.clone()))
            .unwrap_or(Vec::new(&env));

        for v in vouches.iter() {
            let yield_amount = v.stake * YIELD_BPS / 10_000;
            token.transfer(
                &env.current_contract_address(),
                &v.voucher,
                &(v.stake + yield_amount),
            );
        }

        loan.repaid = true;
        env.storage()
            .persistent()
            .set(&DataKey::Loan(borrower), &loan);
    }

    /// Admin marks a loan defaulted; 50% of each voucher's stake is slashed.
    pub fn slash(env: Env, borrower: Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();

        let mut loan: LoanRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Loan(borrower.clone()))
            .expect("no active loan");

        assert!(!loan.repaid, "loan already repaid");
        assert!(!loan.defaulted, "already defaulted");

        let token = Self::token(&env);
        let vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower.clone()))
            .unwrap_or(Vec::new(&env));

        for v in vouches.iter() {
            let slash_amount = v.stake * SLASH_BPS / 10_000;
            let returned = v.stake - slash_amount;
            // Return remaining 50% to voucher; slashed half stays in contract.
            if returned > 0 {
                token.transfer(&env.current_contract_address(), &v.voucher, &returned);
            }
        }

        loan.defaulted = true;
        env.storage()
            .persistent()
            .set(&DataKey::Loan(borrower), &loan);
    }

    // ── Views ─────────────────────────────────────────────────────────────────

    pub fn get_loan(env: Env, borrower: Address) -> Option<LoanRecord> {
        env.storage().persistent().get(&DataKey::Loan(borrower))
    }

    pub fn get_vouches(env: Env, borrower: Address) -> Vec<VouchRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Vouches(borrower))
            .unwrap_or(Vec::new(&env))
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn token(env: &Env) -> token::Client {
        let addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized");
        token::Client::new(env, &addr)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::Address as _,
        token::{Client as TokenClient, StellarAssetClient},
        Address, Env,
    };

    fn setup(env: &Env) -> (Address, Address, Address, Address, Address) {
        env.mock_all_auths();

        let admin = Address::generate(env);
        let borrower = Address::generate(env);
        let voucher = Address::generate(env);

        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let token_admin = StellarAssetClient::new(env, &token_id.address());
        token_admin.mint(&voucher, &10_000_000);

        let contract_id = env.register_contract(None, QuorumCreditContract);
        token_admin.mint(&contract_id, &50_000_000);

        QuorumCreditContractClient::new(env, &contract_id)
            .initialize(&admin, &token_id.address());

        (contract_id, token_id.address(), admin, borrower, voucher)
    }

    #[test]
    fn test_vouch_and_loan_disbursed() {
        let env = Env::default();
        let (contract_id, _token_addr, _admin, borrower, voucher) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        client.vouch(&voucher, &borrower, &1_000_000);
        client.request_loan(&borrower, &500_000, &1_000_000);

        let loan = client.get_loan(&borrower).unwrap();
        assert_eq!(loan.amount, 500_000);
        assert!(!loan.repaid);
        assert!(!loan.defaulted);
    }

    #[test]
    fn test_repay_gives_voucher_yield() {
        let env = Env::default();
        let (contract_id, token_addr, _admin, borrower, voucher) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);
        let token = TokenClient::new(&env, &token_addr);

        client.vouch(&voucher, &borrower, &1_000_000);
        client.request_loan(&borrower, &500_000, &1_000_000);
        client.repay(&borrower);

        assert_eq!(token.balance(&voucher), 10_020_000);
    }

    #[test]
    fn test_slash_burns_half_stake() {
        let env = Env::default();
        let (contract_id, token_addr, _admin, borrower, voucher) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);
        let token = TokenClient::new(&env, &token_addr);

        client.vouch(&voucher, &borrower, &1_000_000);
        client.request_loan(&borrower, &500_000, &1_000_000);
        client.slash(&borrower);

        assert_eq!(token.balance(&voucher), 9_500_000);
        assert!(client.get_loan(&borrower).unwrap().defaulted);
    }

    #[test]
    fn test_request_loan_underfunded_contract() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);
        let voucher = Address::generate(&env);

        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let token_admin = StellarAssetClient::new(&env, &token_id.address());
        // Give voucher enough to stake but do NOT pre-fund the contract beyond the stake.
        token_admin.mint(&voucher, &10_000_000);

        let contract_id = env.register_contract(None, QuorumCreditContract);
        // Contract balance starts at 0; after vouch it will hold 1_000_000.
        // Request a loan larger than the contract balance to trigger InsufficientFunds.

        QuorumCreditContractClient::new(&env, &contract_id)
            .initialize(&admin, &token_id.address());

        let client = QuorumCreditContractClient::new(&env, &contract_id);
        // Stake 1_000_000 — contract now holds exactly 1_000_000.
        client.vouch(&voucher, &borrower, &1_000_000);

        // Request 2_000_000 which exceeds the contract's 1_000_000 balance.
        let result = client.try_request_loan(&borrower, &2_000_000, &1_000_000);
        assert_eq!(
            result,
            Err(Ok(ContractError::InsufficientFunds)),
            "expected InsufficientFunds error when contract balance < loan amount"
        );
    }
}
