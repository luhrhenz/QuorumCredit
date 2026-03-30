#!/bin/bash
# setup_branch_protection.sh — Automatically configures branch protection for main.
#
# Requirements:
#   - GitHub CLI (gh) installed and authenticated: 'gh auth login'
#   - You must have admin access to the repository.

set -euo pipefail

# Find current repo dynamically: "owner/repo"
REPO=$(gh repo view --json nameWithOwner -q ".nameWithOwner")

echo "Configuring branch protection for $REPO (main branch) ..."

gh api --method PUT -H "Accept: application/vnd.github+json" \
  "/repos/$REPO/branches/main/protection" \
  --input - <<EOF
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "Rustfmt",
      "Clippy",
      "Cargo Check (wasm32)",
      "Tests",
      "Deploy to Testnet"
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": true,
    "required_approving_review_count": 1
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF

echo "✅ Branch protection enabled successfully on main!"
