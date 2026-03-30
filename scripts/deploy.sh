#!/bin/bash
# deploy.sh — Automated testnet deployment for QuorumCredit.
#
# Builds the WASM artifact, deploys the contract to the target Stellar network,
# and prints the resulting contract ID. Reads all configuration from a .env file
# or from environment variables already set in the shell.
#
# Usage:
#   ./scripts/deploy.sh [--network <network>]
#
# Options:
#   --network   Override the NETWORK value from .env (e.g. testnet, mainnet)
#
# Required environment variables (or .env entries):
#   DEPLOYER_SECRET_KEY — Secret key of the deployer account (S...)
#   NETWORK             — Target Stellar network (default: testnet)
#
# Optional environment variables:
#   ADMIN_ADDRESS       — Admin account address (G...) — used only for reference output
#   TOKEN_CONTRACT      — XLM token contract address (C...) — used only for reference output
#
# Example:
#   DEPLOYER_SECRET_KEY="SB..." NETWORK=testnet ./scripts/deploy.sh
#
# After a successful deploy, run initialize.sh to complete setup:
#   ./scripts/initialize.sh <CONTRACT_ID> <DEPLOYER_ADDRESS> <THRESHOLD> <TOKEN> <ADMIN...>

set -euo pipefail

# ── Resolve script and project root ───────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WASM_PATH="$PROJECT_ROOT/target/wasm32-unknown-unknown/release/quorum_credit.wasm"

# ── Load .env if present ───────────────────────────────────────────────────────

ENV_FILE="$PROJECT_ROOT/.env"
if [ -f "$ENV_FILE" ]; then
    # Export only lines that are valid KEY=VALUE assignments; skip comments and blanks
    set -o allexport
    # shellcheck source=/dev/null
    source "$ENV_FILE"
    set +o allexport
    echo "Loaded config from $ENV_FILE"
fi

# ── Parse CLI arguments (override .env) ───────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --network)
            NETWORK="${2:?'--network requires a value'}"
            shift 2
            ;;
        *)
            echo "Error: Unknown argument: $1" >&2
            echo "Usage: ./scripts/deploy.sh [--network <network>]" >&2
            exit 1
            ;;
    esac
done

# ── Apply defaults ─────────────────────────────────────────────────────────────

NETWORK="${NETWORK:-testnet}"

# ── Validate required variables ───────────────────────────────────────────────

if [ -z "${DEPLOYER_SECRET_KEY:-}" ]; then
    echo "Error: DEPLOYER_SECRET_KEY is not set." >&2
    echo "Set it in .env or export it before running this script." >&2
    exit 1
fi

if [[ "$DEPLOYER_SECRET_KEY" != S* ]]; then
    echo "Error: DEPLOYER_SECRET_KEY must start with 'S' (got: ${DEPLOYER_SECRET_KEY:0:4}...)." >&2
    exit 1
fi

if [[ "$NETWORK" != "testnet" && "$NETWORK" != "mainnet" ]]; then
    echo "Error: NETWORK must be 'testnet' or 'mainnet' (got: $NETWORK)." >&2
    exit 1
fi

# ── Mainnet safety prompt ──────────────────────────────────────────────────────

if [ "$NETWORK" = "mainnet" ]; then
    echo "WARNING: You are about to deploy to MAINNET." >&2
    echo "  Ensure all tests pass and a security audit has been completed." >&2
    echo "" >&2
    read -r -p "Type 'yes' to confirm mainnet deployment: " CONFIRM
    if [ "$CONFIRM" != "yes" ]; then
        echo "Aborted." >&2
        exit 1
    fi
fi

# ── Check dependencies ─────────────────────────────────────────────────────────

if ! command -v cargo &>/dev/null; then
    echo "Error: 'cargo' not found. Install Rust: https://rustup.rs" >&2
    exit 1
fi

if ! command -v stellar &>/dev/null; then
    echo "Error: 'stellar' CLI not found. Install with: cargo install --locked stellar-cli" >&2
    exit 1
fi

if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    echo "Error: wasm32-unknown-unknown target not installed." >&2
    echo "Run: rustup target add wasm32-unknown-unknown" >&2
    exit 1
fi

# ── Step 1: Build WASM ─────────────────────────────────────────────────────────

echo ""
echo "Step 1/2 — Building WASM..."
(cd "$PROJECT_ROOT" && cargo build --target wasm32-unknown-unknown --release --quiet)

if [ ! -f "$WASM_PATH" ]; then
    echo "Error: WASM artifact not found at $WASM_PATH after build." >&2
    exit 1
fi

WASM_SIZE=$(du -sh "$WASM_PATH" | cut -f1)
echo "Build successful — $WASM_PATH ($WASM_SIZE)"

# ── Step 2: Deploy contract ────────────────────────────────────────────────────

echo ""
echo "Step 2/2 — Deploying contract to $NETWORK..."

CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_PATH" \
  --source "$DEPLOYER_SECRET_KEY" \
  --network "$NETWORK")

if [ -z "$CONTRACT_ID" ]; then
    echo "Error: Deploy command returned an empty contract ID." >&2
    exit 1
fi

if [[ "$CONTRACT_ID" != C* ]]; then
    echo "Error: Unexpected contract ID format (got: $CONTRACT_ID). Expected a value starting with 'C'." >&2
    exit 1
fi

# ── Output summary ─────────────────────────────────────────────────────────────

echo ""
echo "Deployment successful."
echo "  Network     : $NETWORK"
echo "  Contract ID : $CONTRACT_ID"
[ -n "${ADMIN_ADDRESS:-}" ]  && echo "  Admin       : $ADMIN_ADDRESS"
[ -n "${TOKEN_CONTRACT:-}" ] && echo "  Token       : $TOKEN_CONTRACT"
echo ""
echo "Next step — initialize the contract (use the SAME deployer key):"
echo ""
echo "  ./scripts/initialize.sh \\"
echo "    $CONTRACT_ID \\"
echo "    <DEPLOYER_ADDRESS> \\"
echo "    <ADMIN_THRESHOLD> \\"
echo "    ${TOKEN_CONTRACT:-<TOKEN_CONTRACT>} \\"
echo "    ${ADMIN_ADDRESS:-<ADMIN_ADDRESS>}"
echo ""
echo "CONTRACT_ID=$CONTRACT_ID"
