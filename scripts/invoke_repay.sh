#!/bin/bash
# invoke_repay.sh — Repay an active loan on the QuorumCredit contract.
#
# The borrower repays their outstanding principal + yield. On full repayment
# each voucher receives their original stake plus 2% yield (configurable via
# Config.yield_bps).
#
# Usage:
#   ./scripts/invoke_repay.sh <borrower> <payment>
#
# Arguments:
#   borrower  - Stellar address of the borrower repaying the loan
#   payment   - Amount to repay in stroops (must be > 0 and <= outstanding balance)
#
# Environment Variables:
#   CONTRACT_ID - Deployed QuorumCredit contract ID
#   SOURCE_KEY  - Secret key of the account signing the transaction (borrower)
#   NETWORK     - Stellar network: testnet | mainnet
#
# Example:
#   CONTRACT_ID=C... SOURCE_KEY=S... NETWORK=testnet \
#     ./scripts/invoke_repay.sh GBORROWER... 102000

set -e

if [ $# -ne 2 ]; then
    echo "Error: expected 2 arguments."
    echo "Usage: ./scripts/invoke_repay.sh <borrower> <payment>"
    exit 1
fi

BORROWER="$1"
PAYMENT="$2"

if ! [[ "$PAYMENT" =~ ^[0-9]+$ ]]; then
    echo "Error: payment must be a positive integer (stroops)."
    exit 1
fi

if [ -z "$CONTRACT_ID" ] || [ -z "$SOURCE_KEY" ] || [ -z "$NETWORK" ]; then
    echo "Error: CONTRACT_ID, SOURCE_KEY, and NETWORK must be set."
    exit 1
fi

echo "Invoking repay..."
echo "  Borrower : $BORROWER"
echo "  Payment  : $PAYMENT stroops"
echo "  Network  : $NETWORK"
echo

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE_KEY" \
  --network "$NETWORK" \
  -- repay \
  --borrower "$BORROWER" \
  --payment "$PAYMENT"
