# Requirements Document

## Introduction

This feature adds a timelock delay to admin-initiated slash operations in the QuorumCredit CosmWasm smart contract. Currently, admins can slash a borrower's vouchers immediately. The timelock introduces a mandatory waiting period between proposing a slash and executing it, giving borrowers and vouchers time to respond (e.g., repay the loan, dispute the action, or withdraw). The contract already defines `TimelockProposal`, `TimelockAction::Slash`, `TIMELOCK_DELAY` (24h), `TIMELOCK_EXPIRY` (72h), and the relevant `DataKey` variants — this feature wires them into the admin slash flow.

## Glossary

- **Admin**: An address registered in `Config.admins` that has been authorized to perform privileged operations.
- **Admin_Threshold**: The minimum number of admin signers required to authorize a multi-sig admin action.
- **Borrower**: An address that has an active loan in the contract.
- **Slash**: The act of marking a loan as defaulted and redistributing a percentage of voucher stakes to the slash treasury.
- **Slash_Proposal**: A `TimelockProposal` with `TimelockAction::Slash(borrower)` stored under `DataKey::Timelock(proposal_id)`.
- **Timelock_Delay**: The minimum number of seconds that must elapse between proposal creation and execution (`TIMELOCK_DELAY = 86400`, i.e. 24 hours).
- **Timelock_Expiry**: The maximum number of seconds after `eta` within which a proposal may be executed before it expires (`TIMELOCK_EXPIRY = 259200`, i.e. 72 hours).
- **ETA**: The earliest timestamp at which a proposal may be executed (`proposal.created_at + TIMELOCK_DELAY`).
- **Proposal_ID**: A monotonically increasing `u64` counter stored under `DataKey::TimelockCounter`.
- **Contract**: The `QuorumCreditContract` Soroban smart contract.

## Requirements

### Requirement 1: Propose Slash

**User Story:** As an admin, I want to propose a slash against a borrower, so that the borrower has time to respond before their vouchers are penalized.

#### Acceptance Criteria

1. WHEN admins call `propose_slash` with a valid borrower address and sufficient admin signers, THE Contract SHALL create a new `Slash_Proposal` with a unique `Proposal_ID`, set `eta` to `current_timestamp + TIMELOCK_DELAY`, and store it under `DataKey::Timelock(proposal_id)`.
2. WHEN `propose_slash` is called, THE Contract SHALL increment `DataKey::TimelockCounter` and use the new value as the `Proposal_ID`.
3. WHEN `propose_slash` is called, THE Contract SHALL set `executed = false` and `cancelled = false` on the new proposal.
4. WHEN `propose_slash` is called, THE Contract SHALL emit an event with topic `("timelock", "proposed")` and data `(proposal_id, borrower, eta)`.
5. IF the number of admin signers is less than `Admin_Threshold`, THEN THE Contract SHALL reject the call with `UnauthorizedCaller`.
6. IF the borrower does not have an active loan at the time of proposal, THEN THE Contract SHALL reject the call with `NoActiveLoan`.
7. IF the contract is paused, THEN THE Contract SHALL reject the call with `ContractPaused`.
8. THE Contract SHALL allow multiple concurrent `Slash_Proposal` entries for the same borrower (each gets a distinct `Proposal_ID`).

### Requirement 2: Execute Slash

**User Story:** As an admin, I want to execute a previously proposed slash after the timelock delay has elapsed, so that the slash is enforced once the response window has closed.

#### Acceptance Criteria

1. WHEN admins call `execute_slash` with a valid `Proposal_ID` and sufficient admin signers, and the current timestamp is greater than or equal to `proposal.eta`, THE Contract SHALL execute the slash logic: mark the loan as `Defaulted`, distribute slashed stakes to the treasury, return remaining stakes to vouchers, and set `proposal.executed = true`.
2. WHEN `execute_slash` succeeds, THE Contract SHALL emit an event with topic `("timelock", "executed")` and data `(proposal_id, borrower, total_slashed)`.
3. IF `Proposal_ID` does not exist in storage, THEN THE Contract SHALL return `TimelockNotFound`.
4. IF the current timestamp is less than `proposal.eta`, THEN THE Contract SHALL return `TimelockNotReady`.
5. IF the current timestamp is greater than `proposal.eta + TIMELOCK_EXPIRY`, THEN THE Contract SHALL return `TimelockExpired`.
6. IF `proposal.executed` is already `true`, THEN THE Contract SHALL return `SlashAlreadyExecuted`.
7. IF `proposal.cancelled` is `true`, THEN THE Contract SHALL return `InvalidStateTransition`.
8. IF the borrower no longer has an active loan at execution time (e.g., they repaid), THEN THE Contract SHALL return `NoActiveLoan` and leave the proposal unexecuted.
9. IF the contract is paused, THEN THE Contract SHALL reject the call with `ContractPaused`.
10. WHEN `execute_slash` succeeds, THE Contract SHALL burn one reputation point via the `ReputationNft` contract if one is configured, consistent with the existing slash behavior.
11. WHEN `execute_slash` succeeds, THE Contract SHALL increment `DataKey::DefaultCount(borrower)` by 1.

### Requirement 3: Cancel Slash Proposal

**User Story:** As an admin, I want to cancel a pending slash proposal, so that I can retract a mistaken or resolved proposal before it is executed.

#### Acceptance Criteria

1. WHEN admins call `cancel_slash` with a valid `Proposal_ID` and sufficient admin signers, THE Contract SHALL set `proposal.cancelled = true` and persist the updated proposal.
2. WHEN `cancel_slash` succeeds, THE Contract SHALL emit an event with topic `("timelock", "cancelled")` and data `(proposal_id, borrower)`.
3. IF `Proposal_ID` does not exist in storage, THEN THE Contract SHALL return `TimelockNotFound`.
4. IF `proposal.executed` is already `true`, THEN THE Contract SHALL return `SlashAlreadyExecuted`.
5. IF `proposal.cancelled` is already `true`, THEN THE Contract SHALL return `InvalidStateTransition`.
6. IF the number of admin signers is less than `Admin_Threshold`, THEN THE Contract SHALL reject the call with `UnauthorizedCaller`.

### Requirement 4: Query Slash Proposal

**User Story:** As a borrower or observer, I want to query a slash proposal by ID, so that I can monitor pending actions against my account.

#### Acceptance Criteria

1. WHEN `get_slash_proposal` is called with a valid `Proposal_ID`, THE Contract SHALL return the `TimelockProposal` stored under `DataKey::Timelock(proposal_id)`.
2. IF `Proposal_ID` does not exist, THEN THE Contract SHALL return `None`.

### Requirement 5: Timelock Delay Configuration

**User Story:** As an admin, I want the timelock delay to be a configurable protocol parameter, so that governance can adjust the response window without a contract upgrade.

#### Acceptance Criteria

1. THE Contract SHALL read the timelock delay from `Config.timelock_delay` when computing `eta` in `propose_slash`, falling back to the `TIMELOCK_DELAY` constant if the field is absent or zero.
2. THE Contract SHALL read the timelock expiry window from `Config.timelock_expiry` when checking expiry in `execute_slash`, falling back to the `TIMELOCK_EXPIRY` constant if the field is absent or zero.
3. WHEN `set_config` is called with a `timelock_delay` value of `0`, THE Contract SHALL treat `0` as "use the default constant" and SHALL NOT store `0` as an override.
