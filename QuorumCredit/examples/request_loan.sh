#!/bin/bash

# Request a microloan if the total vouched stake meets the threshold.
# This command invokes the 'request_loan' function on the QuorumCredit contract.
#
# Note: Duration and Purpose are not currently stored in the contract state
# but are included in the help description for future extensions.
#
# Parameters:
#   $1 - Borrower address
#   $2 - Loan amount (In stroops)
#   $3 - Threshold (Minimum total stake required)

if [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ]; then
    echo "Usage: ./request_loan.sh <borrower_address> <amount> <threshold>"
    echo "Requires environment variables: CONTRACT_ID, SOURCE_KEY, NETWORK"
    exit 1
fi

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE_KEY" \
  --network "$NETWORK" \
  -- request_loan \
  --borrower "$1" \
  --amount "$2" \
  --threshold "$3"
