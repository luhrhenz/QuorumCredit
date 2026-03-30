# Invalid Token Address Bugfix Design

## Overview

`initialize` accepts any `Address` as the `token` parameter and stores it in `Config` without
validation. The first downstream token operation (`vouch`, `request_loan`, `repay`, etc.) then
calls `token::Client::new(&env, &cfg.token)` and invokes a method on it. If the address does not
implement the SEP-41 token interface the Soroban host traps with a generic host-function error,
giving the caller no actionable information.

The fix calls `token::Client::balance` on the provided address inside `initialize` before any
state is written. If the address is not a valid token contract the call traps and the transaction
is rolled back — no state is stored and the error surfaces at initialization time rather than
later.

## Glossary

- **Bug_Condition (C)**: The condition that triggers the bug — `initialize` is called with a
  `token` address that does not implement the SEP-41 token interface.
- **Property (P)**: The desired behavior when the bug condition holds — `initialize` SHALL reject
  the call (trap / return error) before writing any state.
- **Preservation**: All behavior for inputs where the bug condition does NOT hold must remain
  identical to the original code.
- **`initialize`**: The one-time setup function in `src/lib.rs` that stores `Config` (including
  `token`) in instance storage.
- **`token_client`**: The private helper in `src/lib.rs` that constructs a `token::Client` from
  `cfg.token`; used by every token-dependent operation.
- **SEP-41 token interface**: The Soroban standard token interface; the relevant method for
  validation is `balance(address) -> i128`.
- **valid token address**: An `Address` whose deployed contract exposes the SEP-41 token
  interface.
- **invalid token address**: Any `Address` that does not expose the SEP-41 token interface
  (random account address, wrong contract, undeployed address, etc.).

## Bug Details

### Bug Condition

The bug manifests when `initialize` is called with a `token` address that does not implement the
SEP-41 token interface. The contract stores the address unconditionally, and the first call to
`token_client(&env).transfer(...)` or `token_client(&env).balance(...)` traps the host with no
clear error message.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type InitializeCall { deployer, admins, admin_threshold, token }
  OUTPUT: boolean

  RETURN NOT implementsTokenInterface(input.token)
END FUNCTION

FUNCTION implementsTokenInterface(addr)
  // Returns true only if addr is a deployed contract exposing SEP-41
  RETURN canCall(addr, "balance", [someAddress]) WITHOUT trap
END FUNCTION
```

### Examples

- `initialize(deployer, admins, 1, random_account_address)` — stores the address; next `vouch`
  call traps with a host error. **Expected**: `initialize` itself traps/errors before storing.
- `initialize(deployer, admins, 1, undeployed_contract_id)` — same failure mode.
- `initialize(deployer, admins, 1, wrong_contract_address)` — contract exists but has no
  `balance` entrypoint; traps on first token op. **Expected**: rejected at `initialize`.
- `initialize(deployer, admins, 1, valid_xlm_token_address)` — succeeds today and must continue
  to succeed after the fix.

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- Calling `initialize` with a valid SEP-41 token address must continue to succeed and store the
  full `Config` exactly as before.
- All token-dependent operations (`vouch`, `increase_stake`, `decrease_stake`, `request_loan`,
  `repay`, `slash`, `auto_slash`, `slash_treasury`, `withdraw_vouch`, `claim_expired_loan`) must
  continue to work correctly after a valid initialization.
- The double-initialization guard (`"already initialized"` assertion) must remain in effect.
- Admin validation (`validate_admin_config`) must remain unchanged.

**Scope:**
All calls to `initialize` where `token` IS a valid SEP-41 contract, and all subsequent
token-dependent operations, must be completely unaffected by this fix.

## Hypothesized Root Cause

1. **No upfront interface check**: `initialize` performs no validation on the `token` parameter
   beyond accepting it as an `Address`. The Soroban SDK's `token::Client` is a thin wrapper that
   only fails when a method is actually invoked, so the bad address is silently stored.

2. **Deferred client construction**: `token_client` is called lazily at the point of each token
   operation. There is no eager probe at initialization time to confirm the address is callable.

3. **Opaque host trap**: When the host traps on a missing entrypoint the error message does not
   reference the `token` config field, making diagnosis difficult for integrators.

## Correctness Properties

Property 1: Bug Condition - Invalid Token Address Rejected at Initialize

_For any_ call to `initialize` where `isBugCondition` returns true (i.e. the `token` address
does not implement the SEP-41 token interface), the fixed `initialize` function SHALL trap /
return an error before writing any state to storage, ensuring no `Config` entry is persisted.

**Validates: Requirements 2.1**

Property 2: Preservation - Valid Token Address Initializes Successfully

_For any_ call to `initialize` where `isBugCondition` returns false (i.e. the `token` address
IS a valid SEP-41 contract), the fixed `initialize` function SHALL produce exactly the same
outcome as the original: storing `Config` successfully and allowing all subsequent token
operations to proceed without error.

**Validates: Requirements 3.1, 3.2**

## Fix Implementation

### Changes Required

**File**: `src/lib.rs`

**Function**: `initialize`

**Specific Changes**:

1. **Add `InvalidTokenAddress` error variant**: Add a new variant to `ContractError` so the
   rejection is a typed, documented error rather than a raw panic.
   ```rust
   InvalidTokenAddress = 15,
   ```

2. **Probe the token interface before writing state**: After `validate_admin_config` and before
   the first `env.storage()` write, construct a `token::Client` and call `balance` on the
   contract's own address as a probe. If the call traps the transaction rolls back automatically;
   alternatively, wrap it to surface `InvalidTokenAddress`.
   ```rust
   // Validate token implements SEP-41 before storing any state.
   token::Client::new(&env, &token).balance(&env.current_contract_address());
   ```
   Because Soroban cross-contract calls trap on missing entrypoints and the transaction is
   atomically rolled back, this single line is sufficient — no state will have been written yet
   at this point in the function.

3. **No other changes required**: The rest of `initialize`, all other functions, and the
   `token_client` helper remain unchanged.

## Testing Strategy

### Validation Approach

Two-phase approach: first run exploratory tests against the unfixed code to confirm the bug and
understand the failure mode, then verify the fix satisfies both correctness properties.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bug on the UNFIXED code and confirm the
root cause (no upfront validation in `initialize`).

**Test Plan**: Call `initialize` with a non-token address (e.g. a plain account address or a
contract that does not implement SEP-41), then call `vouch`. Assert that `initialize` itself
should have failed. Run on unfixed code to observe that `initialize` succeeds and `vouch` panics.

**Test Cases**:
1. **Random account as token**: `initialize(deployer, admins, 1, account_address)` then `vouch`
   — will panic on unfixed code at the `transfer` call inside `vouch`.
2. **Undeployed contract id as token**: `initialize` succeeds, first token op traps.
3. **Wrong contract (no SEP-41) as token**: `initialize` succeeds, first token op traps.
4. **Valid token, then check no regression**: `initialize` with real token succeeds on both
   fixed and unfixed code.

**Expected Counterexamples**:
- `initialize` returns `Ok(())` for an invalid token address (should have errored).
- Subsequent `vouch` / `request_loan` traps with a host-level error rather than a typed
  `ContractError`.

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds, the fixed `initialize`
rejects the call before storing state.

**Pseudocode:**
```
FOR ALL input WHERE isBugCondition(input) DO
  result := initialize_fixed(input)
  ASSERT result IS Error (trap or ContractError::InvalidTokenAddress)
  ASSERT env.storage().instance().has(DataKey::Config) == false
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the fixed
`initialize` produces the same outcome as the original.

**Pseudocode:**
```
FOR ALL input WHERE NOT isBugCondition(input) DO
  ASSERT initialize_original(input) == initialize_fixed(input)
  // i.e. both succeed and store identical Config
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many valid token + admin configurations automatically.
- It catches edge cases (threshold == admins.len(), single admin, etc.) that manual tests miss.
- It provides strong guarantees that the probe call does not alter behavior for valid inputs.

**Test Cases**:
1. **Valid token preserves success**: Verify `initialize` with a real mock token still stores
   `Config` with the correct `token` field after the fix.
2. **Downstream operations unaffected**: After fixed `initialize` with valid token, `vouch` /
   `repay` / `slash` all behave identically to the pre-fix baseline.
3. **Double-init guard preserved**: Second call to `initialize` still panics with
   `"already initialized"` regardless of token validity.

### Unit Tests

- Test `initialize` with an invalid token address returns an error / traps before storing state.
- Test `initialize` with a valid token address stores `Config` with the correct `token` field.
- Test that `Config` is absent from storage after a failed `initialize` (no partial state).
- Test the double-initialization guard still fires after a successful `initialize`.

### Property-Based Tests

- Generate random `(admins, threshold, valid_token)` tuples and assert `initialize` always
  succeeds and stores the expected `Config` (preservation of Property 2).
- Generate random invalid addresses and assert `initialize` always rejects them before writing
  state (coverage of Property 1).
- Generate random valid configurations and assert all token-dependent operations produce the
  same results before and after the fix (full preservation sweep).

### Integration Tests

- Full flow: `initialize` → `vouch` → `request_loan` → `repay` with a valid token; assert no
  regressions.
- Negative flow: `initialize` with invalid token → assert error → assert no state stored →
  re-initialize with valid token → assert success.
- Admin operations (`slash`, `slash_treasury`) after valid `initialize`; assert unchanged
  behavior.
