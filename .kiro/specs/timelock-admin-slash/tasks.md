# Implementation Plan: Timelock on Admin Slash Operations

## Overview

Wire the existing `TimelockProposal` / `TimelockAction::Slash` types into the admin slash flow by implementing `propose_slash`, `execute_slash`, `cancel_slash`, and `get_slash_proposal`. Extend `Config` with `timelock_delay` and `timelock_expiry` fields. No new error codes or storage keys are needed — all scaffolding already exists in `types.rs` and `errors.rs`.

## Tasks

- [ ] 1. Extend `Config` with timelock fields and add resolver helpers
  - Add `timelock_delay: u64` and `timelock_expiry: u64` to the `Config` struct in `src/types.rs`
  - Both fields default to `0` (meaning "use the constant fallback")
  - Add `effective_timelock_delay(cfg: &Config) -> u64` and `effective_timelock_expiry(cfg: &Config) -> u64` helper functions in `src/helpers.rs`
  - Update any existing `Config` construction sites in `lib.rs` (`initialize`) and tests to include the new fields (set to `0`)
  - Run `cargo check` to confirm no compilation errors
  - _Requirements: 5.1, 5.2, 5.3_

- [ ] 2. Implement `propose_slash` in `src/admin.rs`
  - [ ] 2.1 Implement `propose_slash(env, admin_signers, borrower) -> Result<u64, ContractError>`
    - Call `require_not_paused`, `require_admin_approval`
    - Verify borrower has an active loan via `get_active_loan_record`; return `NoActiveLoan` if not
    - Read and increment `DataKey::TimelockCounter`
    - Compute `eta = env.ledger().timestamp() + effective_timelock_delay(&cfg)`
    - Store `TimelockProposal { id, action: TimelockAction::Slash(borrower), proposer: admin_signers.get(0), eta, executed: false, cancelled: false }` under `DataKey::Timelock(id)`
    - Emit event `("timelock", "proposed")` with data `(proposal_id, borrower, eta)`
    - Return `Ok(proposal_id)`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8_

  - [ ]* 2.2 Write property test: proposal storage round-trip (Property 1)
    - For 100 random borrowers with active loans, call `propose_slash` and assert stored proposal has correct `eta`, `executed=false`, `cancelled=false`, `action=Slash(borrower)`
    - // Feature: timelock-admin-slash, Property 1: Proposal storage round-trip
    - _Requirements: 1.1, 1.3_

  - [ ]* 2.3 Write property test: proposal ID monotonicity (Property 2)
    - For N successive `propose_slash` calls (including multiple for the same borrower), assert each returned ID is exactly previous + 1
    - // Feature: timelock-admin-slash, Property 2: Proposal ID monotonicity
    - _Requirements: 1.2, 1.8_

  - [ ]* 2.4 Write property test: insufficient signers rejected (Property 3)
    - For random threshold values, call `propose_slash` with fewer signers than threshold and assert error + no storage mutation
    - // Feature: timelock-admin-slash, Property 3: Insufficient signers are rejected
    - _Requirements: 1.5_

  - [ ]* 2.5 Write property test: no active loan rejected (Property 4)
    - For random borrower addresses with no active loan, assert `NoActiveLoan` and counter unchanged
    - // Feature: timelock-admin-slash, Property 4: Proposal requires active loan
    - _Requirements: 1.6_

- [ ] 3. Implement `execute_slash` in `src/admin.rs`
  - [ ] 3.1 Implement `execute_slash(env, admin_signers, proposal_id) -> Result<(), ContractError>`
    - Call `require_not_paused`, `require_admin_approval`
    - Load proposal from `DataKey::Timelock(proposal_id)`; return `TimelockNotFound` if absent
    - Return `SlashAlreadyExecuted` if `proposal.executed`
    - Return `InvalidStateTransition` if `proposal.cancelled`
    - Extract `borrower` from `TimelockAction::Slash(borrower)`
    - Check `now >= proposal.eta`; return `TimelockNotReady` if not
    - Check `now <= proposal.eta + effective_timelock_expiry(&cfg)`; return `TimelockExpired` if not
    - Verify borrower still has active loan; return `NoActiveLoan` if not
    - Execute slash logic (reuse internal helper from `governance.rs`'s `execute_slash` or inline): mark loan `Defaulted`, slash voucher stakes, add to `SlashTreasury`, remove `ActiveLoan` and `Vouches` keys, increment `DefaultCount`
    - Burn reputation NFT if configured
    - Set `proposal.executed = true`, persist proposal
    - Emit event `("timelock", "executed")` with data `(proposal_id, borrower, total_slashed)`
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10, 2.11_

  - [ ]* 3.2 Write property test: execute success invariants (Property 5)
    - For 100 random proposals in the executable window, assert `executed=true`, loan `Defaulted`, `DefaultCount` incremented by 1, `SlashTreasury` increased
    - // Feature: timelock-admin-slash, Property 5: Execute slash success invariants
    - _Requirements: 2.1, 2.11_

  - [ ]* 3.3 Write property test: TimelockNotReady guard (Property 6)
    - For random proposals, attempt execute at `eta - 1`; assert `TimelockNotReady` and no state change
    - // Feature: timelock-admin-slash, Property 6: TimelockNotReady guard
    - _Requirements: 2.4_

  - [ ]* 3.4 Write property test: TimelockExpired guard (Property 7)
    - For random proposals, attempt execute at `eta + TIMELOCK_EXPIRY + 1`; assert `TimelockExpired` and no state change
    - // Feature: timelock-admin-slash, Property 7: TimelockExpired guard
    - _Requirements: 2.5_

  - [ ]* 3.5 Write unit tests for execute error paths
    - `TimelockNotFound` on unknown proposal_id
    - `SlashAlreadyExecuted` on double-execute
    - `InvalidStateTransition` on execute-after-cancel
    - `NoActiveLoan` when borrower repaid between propose and execute
    - Reputation NFT burn on successful execute
    - _Requirements: 2.3, 2.6, 2.7, 2.8, 2.10_

- [ ] 4. Checkpoint — ensure all tests pass
  - Run `cargo check` and `cargo test`; resolve any compilation or test failures before continuing.

- [ ] 5. Implement `cancel_slash` and `get_slash_proposal` in `src/admin.rs`
  - [ ] 5.1 Implement `cancel_slash(env, admin_signers, proposal_id) -> Result<(), ContractError>`
    - Call `require_not_paused`, `require_admin_approval`
    - Load proposal; return `TimelockNotFound` if absent
    - Return `SlashAlreadyExecuted` if `proposal.executed`
    - Return `InvalidStateTransition` if `proposal.cancelled`
    - Set `proposal.cancelled = true`, persist
    - Emit event `("timelock", "cancelled")` with data `(proposal_id, borrower)`
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

  - [ ] 5.2 Implement `get_slash_proposal(env, proposal_id) -> Option<TimelockProposal>`
    - Read and return `DataKey::Timelock(proposal_id)`, or `None`
    - _Requirements: 4.1, 4.2_

  - [ ]* 5.3 Write property test: cancel sets cancelled flag (Property 8)
    - For 100 random pending proposals, call `cancel_slash` and assert `cancelled=true` and loan state unchanged
    - // Feature: timelock-admin-slash, Property 8: Cancel sets cancelled flag
    - _Requirements: 3.1_

  - [ ]* 5.4 Write property test: cancelled proposal cannot be executed (Property 9)
    - For any cancelled proposal, assert `execute_slash` returns `InvalidStateTransition` at any timestamp
    - // Feature: timelock-admin-slash, Property 9: Cancelled proposal cannot be executed
    - _Requirements: 2.7_

  - [ ]* 5.5 Write unit tests for cancel error paths
    - `TimelockNotFound` on unknown proposal_id
    - `SlashAlreadyExecuted` when already executed
    - `InvalidStateTransition` on double-cancel
    - `get_slash_proposal` returns `None` for unknown ID
    - _Requirements: 3.3, 3.4, 3.5, 4.2_

- [ ] 6. Implement configurable timelock parameters property test (Property 10)
  - [ ]* 6.1 Write property test: configurable timelock parameters
    - Set `Config.timelock_delay` to a custom value D; call `propose_slash` and assert `eta = timestamp + D`
    - Set `Config.timelock_expiry` to a custom value E; assert execute succeeds at `eta + E` and fails at `eta + E + 1`
    - // Feature: timelock-admin-slash, Property 10: Configurable timelock parameters
    - _Requirements: 5.1, 5.2_

- [ ] 7. Wire new functions into `lib.rs` contract interface
  - Add `propose_slash`, `execute_slash`, `cancel_slash`, `get_slash_proposal` as public methods on `QuorumCreditContract` in `src/lib.rs`, delegating to `admin::*`
  - _Requirements: 1.1, 2.1, 3.1, 4.1_

- [ ] 8. Final checkpoint — ensure all tests pass
  - Run `cargo check`, `cargo clippy`, and `cargo test`; all must pass with no warnings or failures.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- The existing immediate `slash` function is intentionally preserved for emergency use
- All error codes and storage keys are pre-existing; no schema migrations needed
- `execute_slash` should reuse or call the same internal slash logic as `governance::execute_slash` to avoid duplication — consider extracting a shared `internal_execute_slash(env, borrower)` helper in `helpers.rs`
- Property tests run minimum 100 iterations each using randomized Soroban test environments
