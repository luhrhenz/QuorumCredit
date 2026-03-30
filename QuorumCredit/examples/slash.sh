#!/bin/bash

# Mark a loan as defaulted and slash 50% of each voucher's stake.
# This command invokes the 'slash' function (Admin only).
#
# Parameters:
#   $1 - Defaulting borrower address

if [ -z "$1" ]; then
    echo "Usage: ./slash.sh <borrower_address>"
    echo "Requires environment variables: CONTRACT_ID, SOURCE_KEY, NETWORK"
    exit 1
fi

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE_KEY" \
  --network "$NETWORK" \
  -- slash \
  --borrower "$1"
