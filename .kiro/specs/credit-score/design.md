# Design Document: Credit Score

## Overview

The credit score feature adds a deterministic, on-chain reputation signal to the QuorumCredit lending protocol. It derives a single `u32` score from two counters already maintained in persistent storage — `RepaymentCount` and `DefaultCount` — and exposes it through a new read-only contract function `get_credit_score(borrower)`.

The score formula is:

```
CreditScore = (repayment_count * 10).saturating_sub(default_count * 20)
```

Because both counters are already incremented by the existing `repay` and `execute_slash` / `vote_slash` paths, the implementation is purely additive: no existing logic changes, only a new view function and its test module are introduced.

## Architecture

The feature sits entirely within the existing `QuorumCreditContract` Soroban smart contract. No new contracts, cross-contract calls, or off-chain components are required.

```mermaid
flowchart LR
    subgraph Existing paths
        A[repay()] -->|increments| RC[DataKey::RepaymentCount]
        B[execute_slash()] -->|increments| DC[DataKey::DefaultCount]
    end
    subgraph New read path
        RC --> CS[get_credit_score()]
        DC --> CS
    end
    CS -->|returns u32| Caller
```

The two counters are already written by:
- `loan::repay` — increments `RepaymentCount` on full repayment.
- `governance::execute_slash` (internal) — increments `DefaultCount` on slash execution.

`get_credit_score` only reads these counters; it never writes.

## Components and Interfaces

### New public function: `get_credit_score`

Added to `QuorumCreditContract` in `src/lib.rs`, delegating to a helper in `src/loan.rs` (alongside the existing `repayment_count` and `default_count` helpers).

```rust
// src/lib.rs
pub fn get_credit_score(env: Env, borrower: Address) -> u32 {
    loan::get_credit_score(env, borrower)
}
```

```rust
// src/loan.rs
pub fn get_credit_score(env: Env, borrower: Address) -> u32 {
    let repayments: u32 = env
        .storage()
        .persistent()
        .get(&DataKey::RepaymentCount(borrower.clone()))
        .unwrap_or(0);
    let defaults: u32 = env
        .storage()
        .persistent()
        .get(&DataKey::DefaultCount(borrower))
        .unwrap_or(0);
    (repayments * 10).saturating_sub(defaults * 20)
}
```

### Existing counter writers (unchanged)

| Function | Storage key written | Trigger |
|---|---|---|
| `loan::repay` | `DataKey::RepaymentCount(borrower)` | Full repayment |
| `governance::execute_slash` | `DataKey::DefaultCount(borrower)` | Slash quorum reached or timelock executed |

### Existing counter readers (unchanged)

`repayment_count(borrower)` and `default_count(borrower)` already exist in `lib.rs` and `loan.rs`.

## Data Models

No new storage keys are introduced. The feature reuses:

| Key | Type | Semantics |
|---|---|---|
| `DataKey::RepaymentCount(Address)` | `u32` | Total fully-repaid loans for a borrower. Defaults to 0. |
| `DataKey::DefaultCount(Address)` | `u32` | Total slashed/defaulted loans for a borrower. Defaults to 0. |

Both keys live in **persistent** storage, consistent with the rest of the borrower-scoped data.

The computed score is **not stored** — it is derived on every call to `get_credit_score`. This avoids any possibility of the stored score drifting out of sync with the underlying counters.

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Score formula fidelity

*For any* borrower address, `get_credit_score(borrower)` must equal `(repayment_count(borrower) * 10).saturating_sub(default_count(borrower) * 20)`.

**Validates: Requirements 3.2, 4.1**

### Property 2: Zero-history score is zero

*For any* borrower address that has never repaid or defaulted (both counters are 0), `get_credit_score` must return 0.

**Validates: Requirements 3.3**

### Property 3: Saturation floor — no underflow

*For any* combination of repayment count `r` and default count `d` where `d * 20 > r * 10`, `get_credit_score` must return 0 rather than wrapping or panicking.

**Validates: Requirements 3.4**

### Property 4: Read-only — no state mutation

*For any* borrower address, calling `get_credit_score` must leave `RepaymentCount`, `DefaultCount`, and all other storage keys unchanged.

**Validates: Requirements 3.5**

### Property 5: Repayment increments score by 10

*For any* borrower, after a full repayment the score must increase by exactly 10 compared to the score before that repayment (subject to the saturation floor).

**Validates: Requirements 1.1, 4.2**

### Property 6: Default decrements score by 20 (floored at 0)

*For any* borrower, after a slash the score must decrease by exactly 20 compared to the score before the slash, floored at 0.

**Validates: Requirements 2.1, 4.3**

## Error Handling

`get_credit_score` has no failure modes:

- Missing storage keys are treated as 0 via `unwrap_or(0)` — consistent with the existing `repayment_count` and `default_count` helpers.
- Arithmetic uses `saturating_sub`, so the result is always a valid `u32` with no possibility of panic or wrap-around.
- The function takes no auth, requires no active loan, and reads only persistent storage.

## Testing Strategy

### Dual testing approach

Both unit tests (specific examples and edge cases) and property-based tests (universal properties across generated inputs) are required.

### Unit tests

Located in a new test module `src/credit_score_test.rs`, registered under `#[cfg(test)]` in `lib.rs`.

Specific cases to cover:

- `get_credit_score` returns 0 for a fresh borrower (no history). *(Req 5.1)*
- Score increases by 10 after each successful repayment. *(Req 5.2)*
- Score decreases by 20 after each default, floored at 0. *(Req 5.3)*
- `RepaymentCount` and `DefaultCount` are each incremented exactly once per qualifying event. *(Req 5.4)*
- Score is 0 when defaults dominate (e.g. 0 repayments, 1 default → score = 0). *(Req 3.4)*

### Property-based tests

Use the [`proptest`](https://github.com/proptest-rs/proptest) crate (already common in Rust ecosystems; add to `[dev-dependencies]` in `Cargo.toml`).

Each property test must run a minimum of **100 iterations**.

Tag format for each test: `// Feature: credit-score, Property <N>: <property_text>`

| Property | Test description |
|---|---|
| P1 — Formula fidelity | Generate arbitrary `(r: u32, d: u32)`, set counters directly in storage, assert `get_credit_score == (r*10).saturating_sub(d*20)`. |
| P2 — Zero history | Generate arbitrary borrower address with no storage entries, assert score == 0. |
| P3 — Saturation floor | Generate `(r, d)` where `d*20 > r*10`, assert score == 0. |
| P4 — Read-only | Snapshot all relevant storage keys before and after calling `get_credit_score`, assert no change. |
| P5 — Repayment delta | Generate a borrower with arbitrary prior history, record score, simulate a repayment increment, assert new score == old score + 10 (or capped at u32::MAX). |
| P6 — Default delta | Generate a borrower with arbitrary prior history, record score, simulate a default increment, assert new score == max(0, old score - 20). |

Properties P5 and P6 are comprehensive and subsume the unit-test examples for those cases, but the unit tests are retained for readability and regression anchoring.

**`Cargo.toml` addition:**

```toml
[dev-dependencies]
proptest = "1"
```
