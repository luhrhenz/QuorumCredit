#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Vec};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Yield paid to vouchers on repayment: 2% of their stake.
const YIELD_BPS: i128 = 200;
/// Slash penalty on default: 50% of voucher stake burned.
const SLASH_BPS: i128 = 5000;

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
    pub fn request_loan(env: Env, borrower: Address, amount: i128, threshold: i128) {
        borrower.require_auth();

        let vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower.clone()))
            .unwrap_or(Vec::new(&env));

        let total_stake: i128 = vouches.iter().map(|v| v.stake).sum();
        assert!(total_stake >= threshold, "insufficient trust stake");

        // Send loan amount to borrower.
        Self::token(&env).transfer(&env.current_contract_address(), &borrower, &amount);

        env.storage().persistent().set(
            &DataKey::Loan(borrower.clone()),
            &LoanRecord {
                borrower,
                amount,
                repaid: false,
                defaulted: false,
            },
        );
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

    /// Withdraw a vouch before any loan is active, returning the exact stake to the voucher.
    pub fn withdraw_vouch(env: Env, voucher: Address, borrower: Address) {
        voucher.require_auth();

        // Block withdrawal if a loan record already exists for this borrower.
        assert!(
            env.storage()
                .persistent()
                .get::<DataKey, LoanRecord>(&DataKey::Loan(borrower.clone()))
                .is_none(),
            "loan already active"
        );

        // Load the vouches list; panic if absent.
        let mut vouches: Vec<VouchRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::Vouches(borrower.clone()))
            .expect("vouch not found");

        // Find the index of the matching VouchRecord.
        let idx = vouches
            .iter()
            .position(|v| v.voucher == voucher)
            .expect("vouch not found") as u32;

        let stake = vouches.get(idx).unwrap().stake;
        vouches.remove(idx);

        // Persist updated list or remove the key if empty.
        if vouches.is_empty() {
            env.storage()
                .persistent()
                .remove(&DataKey::Vouches(borrower));
        } else {
            env.storage()
                .persistent()
                .set(&DataKey::Vouches(borrower), &vouches);
        }

        // Return exact stake to voucher.
        Self::token(&env).transfer(&env.current_contract_address(), &voucher, &stake);
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

    // ── withdraw_vouch tests ──────────────────────────────────────────────────

    /// 2.1 Happy path: stake is returned and vouch record is removed.
    /// Requirements: 4.1, 4.2, 5.1, 5.2
    #[test]
    fn test_withdraw_vouch_happy_path() {
        let env = Env::default();
        let (contract_id, token_addr, _admin, borrower, voucher) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);
        let token = TokenClient::new(&env, &token_addr);

        let balance_before = token.balance(&voucher); // 10_000_000
        client.vouch(&voucher, &borrower, &1_000_000);
        client.withdraw_vouch(&voucher, &borrower);

        assert_eq!(token.balance(&voucher), balance_before);
        assert!(client.get_vouches(&borrower).is_empty());
    }

    /// 2.4 Panics with "loan already active" when a LoanRecord exists.
    /// Requirements: 2.1
    #[test]
    #[should_panic(expected = "loan already active")]
    fn test_withdraw_vouch_loan_active_panics() {
        let env = Env::default();
        let (contract_id, _token_addr, _admin, borrower, voucher) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        client.vouch(&voucher, &borrower, &1_000_000);
        client.request_loan(&borrower, &500_000, &1_000_000);
        client.withdraw_vouch(&voucher, &borrower);
    }

    /// 2.5 Panics with "vouch not found" when no matching VouchRecord exists.
    /// Requirements: 3.1
    #[test]
    #[should_panic(expected = "vouch not found")]
    fn test_withdraw_vouch_not_found_panics() {
        let env = Env::default();
        let (contract_id, _token_addr, _admin, borrower, voucher) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        // Never vouched — should panic immediately.
        client.withdraw_vouch(&voucher, &borrower);
    }

    /// 2.6 Only the target VouchRecord is removed when multiple vouchers exist.
    /// Requirements: 5.3
    #[test]
    fn test_withdraw_vouch_isolation() {
        let env = Env::default();
        let (contract_id, token_addr, admin, borrower, voucher1) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        // Mint tokens for a second voucher.
        let voucher2 = Address::generate(&env);
        let token_admin = StellarAssetClient::new(&env, &token_addr);
        token_admin.mint(&voucher2, &10_000_000);
        let _ = admin; // suppress unused warning

        client.vouch(&voucher1, &borrower, &1_000_000);
        client.vouch(&voucher2, &borrower, &2_000_000);

        client.withdraw_vouch(&voucher1, &borrower);

        let remaining = client.get_vouches(&borrower);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining.get(0).unwrap().voucher, voucher2);
    }

    /// 2.7 Vouches key is removed from storage when the last vouch is withdrawn.
    /// Requirements: 5.2
    #[test]
    fn test_withdraw_vouch_removes_storage_key() {
        let env = Env::default();
        let (contract_id, _token_addr, _admin, borrower, voucher) = setup(&env);
        let client = QuorumCreditContractClient::new(&env, &contract_id);

        client.vouch(&voucher, &borrower, &1_000_000);
        client.withdraw_vouch(&voucher, &borrower);

        // get_vouches returns empty vec when key is absent.
        assert!(client.get_vouches(&borrower).is_empty());
    }
}
