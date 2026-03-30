#!/bin/bash
# initialize.sh — Post-deploy contract initialization for QuorumCredit.
#
# Calls the `initialize` function on a deployed Soroban contract.
# The source key used here MUST match the deployer keypair that signed
# the deploy transaction, or `require_auth()` will reject the call.
#
# Usage:
#   ./scripts/initialize.sh <contract_id> <deployer_address> <admin_threshold> <token_contract> <admin1> [admin2 ...]
#
# Arguments:
#   $1 - CONTRACT_ID       — The deployed contract ID (C...)
#   $2 - DEPLOYER_ADDRESS  — The Stellar address of the deployer (G...)
#   $3 - ADMIN_THRESHOLD   — Number of admins required to authorize admin actions (u32)
#   $4 - TOKEN_CONTRACT    — The XLM token contract address (C...)
#   $5+ - ADMIN_ADDRESSES  — One or more admin account addresses (G...)
#
# Required environment variables:
#   SOURCE_KEY — Secret key of the deployer (S...). Must match DEPLOYER_ADDRESS.
#   NETWORK    — Stellar network to target (e.g. testnet, mainnet)
#
# Example (single admin):
#   SOURCE_KEY="SB..." NETWORK=testnet \
#     ./scripts/initialize.sh C... G... 1 C... G...
#
# Example (multisig, threshold 2 of 3):
#   SOURCE_KEY="SB..." NETWORK=testnet \
#     ./scripts/initialize.sh C... G... 2 C... G...admin1 G...admin2 G...admin3

set -euo pipefail

# ── Argument validation ────────────────────────────────────────────────────────

CONTRACT_ID="${1:-}"
DEPLOYER_ADDRESS="${2:-}"
ADMIN_THRESHOLD="${3:-}"
TOKEN_CONTRACT="${4:-}"

if [ -z "$CONTRACT_ID" ] || [ -z "$DEPLOYER_ADDRESS" ] || \
   [ -z "$ADMIN_THRESHOLD" ] || [ -z "$TOKEN_CONTRACT" ]; then
    echo "Error: Missing required arguments." >&2
    echo "" >&2
    echo "Usage: ./scripts/initialize.sh <contract_id> <deployer_address> <admin_threshold> <token_contract> <admin1> [admin2 ...]" >&2
    echo "" >&2
    echo "Required environment variables: SOURCE_KEY, NETWORK" >&2
    exit 1
fi

# Remaining args are admin addresses
shift 4
ADMIN_ADDRESSES=("$@")

if [ "${#ADMIN_ADDRESSES[@]}" -eq 0 ]; then
    echo "Error: At least one admin address is required." >&2
    exit 1
fi

# ── Environment variable validation ───────────────────────────────────────────

if [ -z "${SOURCE_KEY:-}" ]; then
    echo "Error: SOURCE_KEY environment variable is not set." >&2
    echo "Set it to the deployer's secret key (S...) before running this script." >&2
    exit 1
fi

if [ -z "${NETWORK:-}" ]; then
    echo "Error: NETWORK environment variable is not set." >&2
    echo "Set it to the target network (e.g. testnet or mainnet)." >&2
    exit 1
fi

# ── Address format checks ──────────────────────────────────────────────────────

if [[ "$CONTRACT_ID" != C* ]]; then
    echo "Error: CONTRACT_ID must start with 'C' (got: $CONTRACT_ID)." >&2
    exit 1
fi

if [[ "$DEPLOYER_ADDRESS" != G* ]]; then
    echo "Error: DEPLOYER_ADDRESS must start with 'G' (got: $DEPLOYER_ADDRESS)." >&2
    exit 1
fi

if [[ "$TOKEN_CONTRACT" != C* ]]; then
    echo "Error: TOKEN_CONTRACT must start with 'C' (got: $TOKEN_CONTRACT)." >&2
    exit 1
fi

if [[ "$SOURCE_KEY" != S* ]]; then
    echo "Error: SOURCE_KEY must start with 'S' (got: ${SOURCE_KEY:0:4}...)." >&2
    exit 1
fi

if ! [[ "$ADMIN_THRESHOLD" =~ ^[1-9][0-9]*$ ]]; then
    echo "Error: ADMIN_THRESHOLD must be a positive integer (got: $ADMIN_THRESHOLD)." >&2
    exit 1
fi

for addr in "${ADMIN_ADDRESSES[@]}"; do
    if [[ "$addr" != G* ]]; then
        echo "Error: Admin address must start with 'G' (got: $addr)." >&2
        exit 1
    fi
done

if [ "$ADMIN_THRESHOLD" -gt "${#ADMIN_ADDRESSES[@]}" ]; then
    echo "Error: ADMIN_THRESHOLD ($ADMIN_THRESHOLD) cannot exceed the number of admins (${#ADMIN_ADDRESSES[@]})." >&2
    exit 1
fi

# ── Build the --admins JSON array ──────────────────────────────────────────────

# Soroban CLI expects a JSON array for Vec<Address>, e.g. '["G...","G..."]'
ADMINS_JSON="["
for i in "${!ADMIN_ADDRESSES[@]}"; do
    [ "$i" -gt 0 ] && ADMINS_JSON+=","
    ADMINS_JSON+="\"${ADMIN_ADDRESSES[$i]}\""
done
ADMINS_JSON+="]"

# ── Mainnet safety prompt ──────────────────────────────────────────────────────

if [ "$NETWORK" = "mainnet" ]; then
    echo "WARNING: You are about to initialize a contract on MAINNET." >&2
    echo "  Contract  : $CONTRACT_ID" >&2
    echo "  Deployer  : $DEPLOYER_ADDRESS" >&2
    echo "  Admins    : ${ADMIN_ADDRESSES[*]}" >&2
    echo "  Threshold : $ADMIN_THRESHOLD" >&2
    echo "  Token     : $TOKEN_CONTRACT" >&2
    echo "" >&2
    read -r -p "Type 'yes' to confirm: " CONFIRM
    if [ "$CONFIRM" != "yes" ]; then
        echo "Aborted." >&2
        exit 1
    fi
fi

# ── Invoke initialize ──────────────────────────────────────────────────────────

echo "Initializing contract $CONTRACT_ID on $NETWORK..."

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE_KEY" \
  --network "$NETWORK" \
  -- initialize \
  --deployer "$DEPLOYER_ADDRESS" \
  --admins "$ADMINS_JSON" \
  --admin_threshold "$ADMIN_THRESHOLD" \
  --token "$TOKEN_CONTRACT"

echo "Contract initialized successfully."
echo "  Contract  : $CONTRACT_ID"
echo "  Admins    : ${ADMIN_ADDRESSES[*]}"
echo "  Threshold : $ADMIN_THRESHOLD"
echo "  Token     : $TOKEN_CONTRACT"
echo "  Network   : $NETWORK"
