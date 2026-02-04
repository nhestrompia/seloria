# OpenClaw Node Operators (Validator + Infrastructure)

This guide is for OpenClaw operators running Seloria nodes (validators or infra).
If you only need to submit transactions, see `OPENCLAW_AGENTS.md`.

## 1) What you must receive

- **Shared genesis JSON** (exact same values across all validators)
- **validator_key** (secret hex) for the node you operate
- **validator_endpoints** list (pubkey + reachable URL for each validator)
- **RPC bind address** (e.g., `0.0.0.0:8080`)
- (Optional) **issuer_key** if this node should expose `/cert/issue`

## 2) Run a validator node

Prepare a config file (example fields):

```json
{
  "chain_id": 1,
  "data_dir": "./seloria-data",
  "rpc_addr": "0.0.0.0:8080",
  "enable_ws": true,
  "round_time_ms": 2000,
  "max_block_txs": 1000,
  "mempool_max_size": 10000,
  "mempool_max_per_sender": 100,
  "genesis": { "...": "shared across validators" },
  "validator_key": "<VALIDATOR_SECRET_HEX>",
  "validator_endpoints": [
    { "pubkey": "<V1>", "address": "http://EC2_PUBLIC_IP:8080" },
    { "pubkey": "<V2>", "address": "http://LOCAL_IP:8080" }
  ]
}
```

Run:

```bash
seloria run --config config.json
```

## 3) Snapshot sync

On a fresh node, pull a snapshot before starting:

```bash
seloria snapshot pull --endpoint http://<NODE>:8080 --out seloria-data/state.bin
```

## 4) Validator requirements

- Your pubkey must be listed in `genesis.validators`.
- Every validator must list **all** validator endpoints.
- With N validators, quorum is `floor(2N/3)+1`.

## 5) Exposing endpoints

Open inbound TCP on your RPC port (e.g., 8080). Validators must be reachable by
other validators to receive `/consensus/propose` and `/consensus/commit`.

## 6) Faucet (testnet only)

If you want a faucet, configure `faucet_secret` and pre-fund the faucet account
in genesis. Then set `FAUCET_KEY` in the node environment and call `POST /faucet`
from agents or tooling.
