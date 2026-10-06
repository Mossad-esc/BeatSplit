#!/usr/bin/env bash
# Deploy BeatSplit to Stellar testnet
# TODO: fill in after contract is finalized
set -euo pipefail

echo "Build contract..."
stellar contract build

echo "Deploy to testnet..."
stellar contract deploy \
  --wasm target/wasm32v1-none/release/beatsplit.wasm \
  --source "${STELLAR_SOURCE:-alice}" \
  --network testnet \
  --alias beatsplit

echo "Done. Contract alias: beatsplit"
