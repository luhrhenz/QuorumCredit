#!/bin/bash

# Script to invoke the request_loan function on a Stellar smart contract using the Stellar CLI.
#
# Usage: ./scripts/invoke_request_loan.sh <borrower> <amount> <threshold>
#
# Arguments:
#   borrower - The account address of the borrower (string)
#   amount   - The loan amount in stroops (integer)
#   threshold - The minimum total stake required for approval (integer)
#
# Environment Variables Required:
#   CONTRACT_ID - The ID of the deployed smart contract
#   SOURCE_KEY  - The secret key of the source account
#   NETWORK     - The Stellar network (e.g., testnet, mainnet)
#
# Example:
#   ./scripts/invoke_request_loan.sh GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ 1000000 500000

set -e

# Validate arguments
if [ $# -ne 3 ]; then
    echo "Error: Missing required arguments."
    echo "Usage: ./scripts/invoke_request_loan.sh <borrower> <amount> <threshold>"
    echo "  borrower  - Account address of the borrower"
    echo "  amount    - Loan amount in stroops (integer)"
    echo "  threshold - Approval threshold (integer)"
    exit 1
fi

# Assign arguments to readable variables
borrower_address="$1"
loan_amount="$2"
approval_threshold="$3"

# Validate that amount and threshold are integers
if ! [[ "$loan_amount" =~ ^[0-9]+$ ]]; then
    echo "Error: Amount must be a positive integer."
    exit 1
fi

if ! [[ "$approval_threshold" =~ ^[0-9]+$ ]]; then
    echo "Error: Threshold must be a positive integer."
    exit 1
fi

# Check for required environment variables
if [ -z "$CONTRACT_ID" ] || [ -z "$SOURCE_KEY" ] || [ -z "$NETWORK" ]; then
    echo "Error: Missing required environment variables."
    echo "Please set CONTRACT_ID, SOURCE_KEY, and NETWORK."
    exit 1
fi

# Construct and print the command
command="stellar contract invoke \\
  --id \"$CONTRACT_ID\" \\
  --source \"$SOURCE_KEY\" \\
  --network \"$NETWORK\" \\
  -- request_loan \\
  --borrower \"$borrower_address\" \\
  --amount \"$loan_amount\" \\
  --threshold \"$approval_threshold\""

echo "Executing command:"
echo "$command"
echo

# Execute the command
eval "$command"