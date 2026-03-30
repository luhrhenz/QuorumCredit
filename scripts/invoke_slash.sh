#!/bin/bash
# invoke_slash.sh — Mark a loan as defaulted and slash voucher stakes.
#
# Admin-only. Burns Config.slash_bps (default 50%) of each voucher's stake
# and marks the loan as Defaulted. The slashed funds are held in the contract's
# slash treasury.
#
# Usage:
#   ./scripts/invoke_slash.sh <borrower> <admin_signer>
#
# Arguments:
#   borrower      - Stellar address of the defaulting borrower
#   admin_signer  - Stellar address of the admin signing the slash (must be a
#                   registered admin; pass multiple signers separated by commas
#                   if admin_threshold > 1, e.g. "ADDR1,ADDR2")
#
# Environment Variables:
#   CONTRACT_ID - Deployed QuorumCredit contract ID
#   SOURCE_KEY  - Secret key of the admin account signing the transaction
#   NETWORK     - Stellar network: testnet | mainnet
#
# Example (single admin):
#   CONTRACT_ID=C... SOURCE_KEY=S... NETWORK=testnet \
#     ./scripts/invoke_slash.sh GBORROWER... GADMIN...
#
# Example (multisig, threshold = 2):
#   CONTRACT_ID=C... SOURCE_KEY=S... NETWORK=testnet \
#     ./scripts/invoke_slash.sh GBORROWER... "GADMIN1...,GADMIN2..."

set -e

if [ $# -ne 2 ]; then
    echo "Error: expected 2 arguments."
    echo "Usage: ./scripts/invoke_slash.sh <borrower> <admin_signer[,admin_signer...]>"
    exit 1
fi

BORROWER="$1"
RAW_SIGNERS="$2"

if [ -z "$CONTRACT_ID" ] || [ -z "$SOURCE_KEY" ] || [ -z "$NETWORK" ]; then
    echo "Error: CONTRACT_ID, SOURCE_KEY, and NETWORK must be set."
    exit 1
fi

# Build a JSON array from the comma-separated signer list.
# e.g. "ADDR1,ADDR2" → '["ADDR1","ADDR2"]'
ADMIN_SIGNERS_JSON="[$(echo "$RAW_SIGNERS" | sed 's/,/","/g' | sed 's/^/"/;s/$/"/' )]"

echo "Invoking slash..."
echo "  Borrower       : $BORROWER"
echo "  Admin signers  : $ADMIN_SIGNERS_JSON"
echo "  Network        : $NETWORK"
echo

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE_KEY" \
  --network "$NETWORK" \
  -- slash \
  --borrower "$BORROWER" \
  --admin_signers "$ADMIN_SIGNERS_JSON"
