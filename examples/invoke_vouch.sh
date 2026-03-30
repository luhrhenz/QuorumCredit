#!/bin/bash

# Invoke the vouch function on the QuorumCredit contract via the Stellar CLI.
# Vouching stakes XLM on behalf of a borrower, signaling trust and enabling
# them to qualify for a microloan.
#
# Usage:
#   ./invoke_vouch.sh <voucher_address> <borrower_address> <stake_amount>
#
# Parameters:
#   $1 - Voucher address  (the account staking XLM)
#   $2 - Borrower address (the account being vouched for)
#   $3 - Stake amount     (in stroops, e.g. 10000000 = 1 XLM)
#
# Required environment variables:
#   CONTRACT_ID  - The deployed QuorumCredit contract ID
#   SOURCE_KEY   - The Stellar account key used to sign the transaction
#   NETWORK      - The Stellar network to use (e.g. testnet, mainnet)
#
# Example:
#   export CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
#   export SOURCE_KEY=alice
#   export NETWORK=testnet
#   ./invoke_vouch.sh GVOUCHER... GBORROWER... 10000000

if [ -z "$CONTRACT_ID" ] || [ -z "$SOURCE_KEY" ] || [ -z "$NETWORK" ]; then
    echo "Error: CONTRACT_ID, SOURCE_KEY, and NETWORK environment variables must be set."
    exit 1
fi

if [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ]; then
    echo "Usage: ./invoke_vouch.sh <voucher_address> <borrower_address> <stake_amount>"
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
