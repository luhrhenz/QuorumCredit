.PHONY: build test deploy-testnet deploy-mainnet

# ── Config ────────────────────────────────────────────────────────────────────

CONTRACT_DIR := QuorumCredit
WASM_TARGET  := wasm32-unknown-unknown

# ── Targets ───────────────────────────────────────────────────────────────────

## Compile the contract (native + WASM release build)
build:
	cd $(CONTRACT_DIR) && cargo build --target $(WASM_TARGET) --release

## Run the full test suite
test:
	cd $(CONTRACT_DIR) && cargo test

## Deploy to Stellar testnet
deploy-testnet:
	stellar contract deploy \
		--wasm $(CONTRACT_DIR)/target/$(WASM_TARGET)/release/quorum_credit.wasm \
		--network testnet \
		--source $(DEPLOYER_SECRET_KEY)

## Deploy to Stellar mainnet — requires interactive confirmation
deploy-mainnet:
	@echo "WARNING: You are about to deploy to MAINNET."
	@read -p "Are you sure you want to deploy to MAINNET? [y/N]: " confirm && \
		[ "$${confirm:-N}" = "y" ] || [ "$${confirm:-N}" = "Y" ] || \
		(echo "Deployment aborted."; exit 1)
	stellar contract deploy \
		--wasm $(CONTRACT_DIR)/target/$(WASM_TARGET)/release/quorum_credit.wasm \
		--network mainnet \
		--source $(DEPLOYER_SECRET_KEY)
