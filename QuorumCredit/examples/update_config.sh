#!/bin/bash

# Example script demonstrating how to update YIELD_BPS and SLASH_BPS
# This shows the new update_config function that allows selective updates

echo "=== QuorumCredit Config Update Example ==="
echo ""

echo "1. Update only YIELD_BPS to 300 (3%):"
echo "   client.update_config(&admin_signers, &Some(300), &None);"
echo ""

echo "2. Update only SLASH_BPS to 6000 (60%):"
echo "   client.update_config(&admin_signers, &None, &Some(6000));"
echo ""

echo "3. Update both YIELD_BPS and SLASH_BPS:"
echo "   client.update_config(&admin_signers, &Some(400), &Some(7000));"
echo ""

echo "4. No changes (both None):"
echo "   client.update_config(&admin_signers, &None, &None);"
echo ""

echo "Benefits of the new update_config function:"
echo "- Selective updates: Change only what you need"
echo "- Preserves other config values"
echo "- Validates input ranges (yield_bps >= 0, slash_bps 1-10000)"
echo "- Requires admin approval like other admin functions"
echo "- Emits events for tracking changes"
echo ""

echo "The old hardcoded constants are now configurable:"
echo "- DEFAULT_YIELD_BPS = 200 (2%) - used only during initialization"
echo "- DEFAULT_SLASH_BPS = 5000 (50%) - used only during initialization"
echo "- After deployment, use update_config() to change these values"