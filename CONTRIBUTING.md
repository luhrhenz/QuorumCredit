# Contributing to QuorumCredit

Thank you for your interest in contributing to QuorumCredit! We welcome contributions from developers, researchers, and DeFi enthusiasts.

To ensure a smooth collaboration process, please follow these guidelines.

## 🌿 Branch Naming Convention

When creating a new branch, please use one of the following prefixes followed by the issue number or a short description:

| Prefix | Purpose | Example |
|---|---|---|
| `feat/` | New features | `feat/163-add-contributing-guide` |
| `fix/` | Bug fixes | `fix/issue-55-auth-error` |
| `docs/` | Documentation changes | `docs/update-readme-yield` |
| `refactor/` | Code refactoring | `refactor/optimize-vouch-loop` |
| `test/` | Adding/updating tests | `test/add-slash-coverage` |

## 📝 Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification for our commit messages:

`type: description`

Common types include:
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that do not affect the meaning of the code (white-space, formatting, etc.)
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `test`: Adding missing tests or correcting existing tests

**Example:** `feat: add user authentication to request_loan`

## 🚀 Pull Request Process

1. **Fork** the repository and create your branch from `main`.
2. **Code**: Implement your changes.
3. **Test**: Ensure all tests pass locally (see [Testing](#-testing) below).
4. **Style**: Run formatting tools (see [Style Guide](#-style-guide) below).
5. **PR**: Open a Pull Request against the `main` branch.
    - Provide a clear description of the change.
    - Link any related issues (e.g., `Resolves #163`).

## 🧪 Testing

All contributions must pass existing tests. Before submitting your PR, run the following:

```bash
# Run all Soroban contract tests
cargo test

# Run tests with output for debugging
cargo test -- --nocapture
```

If you are adding a new feature, please include corresponding test cases in `src/lib.rs`.

## 🎨 Style Guide

We follow standard Rust formatting conventions. Please run the following before committing:

```bash
cargo fmt --all
```

## ✅ CI Pipeline

Every push and pull request against `main` runs four GitHub Actions jobs defined in `.github/workflows/ci.yml`:

| Job | What it does |
|---|---|
| Rustfmt | `cargo fmt --all -- --check` — fails if any file is not formatted |
| Clippy | `cargo clippy --all-targets -- -D warnings` — fails on any lint warning |
| Cargo Check (wasm32) | `cargo check --target wasm32-unknown-unknown` — verifies the contract compiles to the deployment target |
| Tests | `cargo test` — runs the full test suite on the host target |

All four jobs must pass before a PR can be merged.

## 🪝 Pre-commit Hooks

This repository uses [pre-commit](https://pre-commit.com/) to automatically run `cargo fmt` and `cargo clippy` before every commit, keeping the codebase consistently formatted and lint-free.

### Prerequisites

- Python 3.7+ and `pip`
- Rust toolchain with `rustfmt` and `clippy` components

```bash
rustup component add rustfmt clippy
```

### Setup

Install the `pre-commit` tool and register the hooks with git:

```bash
pip install pre-commit
pre-commit install
```

That's it. From this point on, every `git commit` will automatically run:

1. `cargo fmt --all` — formats all Rust source files
2. `cargo clippy --all-targets -- -D warnings` — lints and fails on any warning

If either check fails, the commit is blocked. Fix the reported issues and re-stage your changes before committing again.

### Running hooks manually

To run all hooks against every file without making a commit:

```bash
pre-commit run --all-files
```

To run a single hook by ID:

```bash
pre-commit run cargo-fmt
pre-commit run cargo-clippy
```

### Skipping hooks (use sparingly)

In exceptional cases (e.g. a WIP commit on a personal branch) you can bypass the hooks:

```bash
git commit --no-verify -m "wip: ..."
```

Do not use `--no-verify` on commits destined for `main` or any PR branch.

---

*Happy Coding! 🚀*

## 🔐 Admin Key Management (Multisig Security)

QuorumCredit uses an M-of-N multisig model for all privileged operations (slash, config, treasury, pause, upgrade). A single compromised key cannot unilaterally act.

### How it works

- `admins: Vec<Address>` — the registered admin set (N keys)
- `admin_threshold: u32` — minimum signatures required (M)
- Every admin function takes `admin_signers: Vec<Address>` and calls `require_admin_approval`, which verifies:
  1. `admin_signers.len() >= admin_threshold`
  2. Every signer is in the registered `admins` set
  3. Every signer has authorised the transaction (`require_auth`)

### Recommended configuration

| Deployment | Recommended setup |
|---|---|
| Testnet / staging | 1-of-1 (single key, for speed) |
| Mainnet small team | 2-of-3 |
| Mainnet production | 3-of-5 |

A good rule of thumb: `threshold = floor(N/2) + 1` (simple majority).

### Key management best practices

1. **Use hardware wallets** (Ledger, Trezor) for every admin key — never hot wallets.
2. **Geographic separation** — store key backups in physically separate, offline locations.
3. **No key sharing** — each admin controls exactly one key; never share private keys.
4. **Rotate keys periodically** using `rotate_admin`. Rotation requires the current quorum to sign, so a single compromised key cannot rotate itself in.
5. **Verify after rotation** — always call `get_admins()` after any key management operation to confirm the expected set.
6. **Never reuse compromised keys** — if a key is suspected compromised, rotate it out immediately using the remaining quorum.
7. **Keep threshold satisfiable** — `remove_admin` is blocked if it would make the threshold unreachable; plan key ceremonies before removing admins.

### Admin management functions

| Function | Description |
|---|---|
| `add_admin(signers, new_admin)` | Add a new admin; threshold unchanged |
| `remove_admin(signers, admin)` | Remove an admin; blocked if threshold would become unsatisfiable |
| `rotate_admin(signers, old, new)` | Atomically swap one key for another; count and threshold preserved |
| `set_admin_threshold(signers, n)` | Update the quorum threshold |
| `set_config(signers, config)` | Full config update including admins and threshold |

All of the above require the current quorum to sign.
