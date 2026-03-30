#!/bin/bash

# Vouch for a borrower by staking XLM.
# This command invokes the 'vouch' function on the QuorumCredit contract.
#
# Parameters:
#   $1 - Voucher address (The person staking)
#   $2 - Borrower address (The subject of the vouch)
#   $3 - Stake amount (In stroops, e.g., 10000000 for 1 XLM)

if [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ]; then
    echo "Usage: ./vouch.sh <voucher_address> <borrower_address> <stake_amount>"
    echo "Requires environment variables: CONTRACT_ID, SOURCE_KEY, NETWORK"
    exit 1
fi

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE_KEY" \
  --network "$NETWORK" \
  -- vouch \
  --voucher "$1" \
  --borrower "$2" \
  --stake "$3"
