# Implementation Plan

- [ ] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** - Invalid Token Address Not Rejected at Initialize
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **DO NOT attempt to fix the test or the code when it fails**
  - **NOTE**: This test encodes the expected behavior - it will validate the fix when it passes after implementation
  - **GOAL**: Surface counterexamples that demonstrate that `initialize` accepts an invalid token address without error
  - **Scoped PBT Approach**: Scope the property to the concrete failing case — call `initialize` with a plain account address (not a SEP-41 contract) as `token`, then assert the call returned an error; on unfixed code it will succeed instead
  - Test that `initialize(deployer, admins, 1, account_address)` traps / returns an error before storing `DataKey::Config` (from Bug Condition in design: `isBugCondition(input)` where `input.token` does not implement SEP-41)
  - After `initialize` returns without error on unfixed code, call `vouch` and observe the host-level panic — document this as the counterexample
  - Assert `env.storage().instance().has(DataKey::Config)` is `false` after a rejected call (expected behavior from design)
  - Run test on UNFIXED code
  - **EXPECTED OUTCOME**: Test FAILS (initialize succeeds when it should not — this proves the bug exists)
  - Document counterexamples found (e.g., "`initialize` returns `Ok(())` for a plain account address; subsequent `vouch` traps with a host error")
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 1.1, 1.2_

- [ ] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** - Valid Token Address Initializes Successfully
  - **IMPORTANT**: Follow observation-first methodology
  - Observe: `initialize(deployer, admins, 1, valid_mock_token)` succeeds and stores `Config` with the correct `token` field on unfixed code
  - Observe: after a valid `initialize`, `vouch` / `repay` / `slash` all execute without error on unfixed code
  - Observe: a second call to `initialize` panics with `"already initialized"` on unfixed code
  - Write property-based test: for all valid `(admins, admin_threshold, valid_token)` tuples where `isBugCondition` is false, `initialize` succeeds and `env.storage().instance().get(DataKey::Config).token == valid_token` (from Preservation Requirements in design: 3.1, 3.2)
  - Include a case asserting the double-init guard still fires after a successful `initialize`
  - Run tests on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2_

- [ ] 3. Fix: reject invalid token address in `initialize`

  - [ ] 3.1 Add `InvalidTokenAddress` error variant to `ContractError`
    - Add `InvalidTokenAddress = 15` to the `ContractError` enum in `src/lib.rs`
    - _Bug_Condition: `isBugCondition(input)` where `NOT implementsTokenInterface(input.token)`_
    - _Requirements: 2.1_

  - [ ] 3.2 Add token probe call inside `initialize` before any state is written
    - In `src/lib.rs`, inside `initialize`, after `Self::validate_admin_config(&admins, admin_threshold)` and before the first `env.storage()` write, add:
      ```rust
      // Validate token implements SEP-41 before storing any state.
      token::Client::new(&env, &token).balance(&env.current_contract_address());
      ```
    - This probe call traps and rolls back the transaction atomically if `token` does not expose the SEP-41 interface — no `Config` entry is persisted
    - No other changes to `initialize` or any other function are required
    - _Bug_Condition: `isBugCondition(input)` where `NOT implementsTokenInterface(input.token)`_
    - _Expected_Behavior: `initialize` traps before writing state; `env.storage().instance().has(DataKey::Config) == false`_
    - _Preservation: all calls where `implementsTokenInterface(input.token)` is true must continue to succeed and store `Config` identically_
    - _Requirements: 2.1, 2.2, 3.1, 3.2_

  - [ ] 3.3 Verify bug condition exploration test now passes
    - **Property 1: Expected Behavior** - Invalid Token Address Rejected at Initialize
    - **IMPORTANT**: Re-run the SAME test from task 1 — do NOT write a new test
    - The test from task 1 encodes the expected behavior: `initialize` with an invalid token address must trap before storing state
    - Run bug condition exploration test from step 1
    - **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed)
    - _Requirements: 2.1_

  - [ ] 3.4 Verify preservation tests still pass
    - **Property 2: Preservation** - Valid Token Address Initializes Successfully
    - **IMPORTANT**: Re-run the SAME tests from task 2 — do NOT write new tests
    - Run preservation property tests from step 2
    - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions)
    - Confirm all downstream token operations (`vouch`, `repay`, `slash`, etc.) still behave identically after the fix

- [ ] 4. Checkpoint - Ensure all tests pass
  - Run the full test suite (`cargo test`)
  - Ensure all tests pass; ask the user if any questions arise
