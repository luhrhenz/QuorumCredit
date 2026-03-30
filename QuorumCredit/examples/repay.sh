#!/bin/bash

# Repay an active loan. Vouchers receive their stake plus 2% yield.
# This command invokes the 'repay' function on the QuorumCredit contract.
#
# Parameters:
#   $1 - Borrower address

if [ -z "$1" ]; then
    echo "Usage: ./repay.sh <borrower_address>"
    echo "Requires environment variables: CONTRACT_ID, SOURCE_KEY, NETWORK"
    exit 1
fi

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE_KEY" \
  --network "$NETWORK" \
  -- repay \
  --borrower "$1"
