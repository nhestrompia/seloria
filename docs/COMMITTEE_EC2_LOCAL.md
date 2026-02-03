# EC2 + Local Committee Test Guide

This guide shows how to run a **two‑validator committee** with one node on EC2 and one locally,
and how an OpenClaw agent can run a validator node.

## 1) Prereqs

- Build/run access to `seloria` (via `cargo run --bin seloria` or a built binary)
- Public EC2 instance with ports open for your RPC/consensus endpoint
- Shared genesis config across all nodes (same chain_id, validators, trusted_issuers)

## 2) Generate keys

```bash
cargo run --bin seloria -- keygen
```

Generate:
- **Validator V1 keypair** (EC2)
- **Validator V2 keypair** (local)
- **Issuer keypair** (optional, for /cert/issue)

## 3) Create shared genesis

Both nodes must have identical genesis settings.

Example (shared fields):
- `chain_id`: same number
- `genesis.timestamp`: same number
- `genesis.validators`: `[V1_PUBKEY, V2_PUBKEY]`
- `genesis.trusted_issuers`: `[ISSUER_PUBKEY]`

## 4) EC2 node config (validator V1)

Create `config-ec2.json`:

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
  "genesis": {
    "timestamp": 1710000000,
    "initial_balances": [],
    "trusted_issuers": ["ISSUER_PUBKEY_HEX"],
    "validators": ["V1_PUBKEY_HEX", "V2_PUBKEY_HEX"]
  },
  "validator_key": "V1_SECRET_HEX",
  "issuer_key": "ISSUER_SECRET_HEX",
  "validator_endpoints": [
    { "pubkey": "V1_PUBKEY_HEX", "address": "http://EC2_PUBLIC_IP:8080" },
    { "pubkey": "V2_PUBKEY_HEX", "address": "http://LOCAL_IP:8081" }
  ]
}
```

Open the EC2 security group for port `8080` (or whatever you bind).

## 5) Local node config (validator V2)

Create `config-local.json`:

```json
{
  "chain_id": 1,
  "data_dir": "./seloria-data-local",
  "rpc_addr": "127.0.0.1:8081",
  "enable_ws": true,
  "round_time_ms": 2000,
  "max_block_txs": 1000,
  "mempool_max_size": 10000,
  "mempool_max_per_sender": 100,
  "genesis": {
    "timestamp": 1710000000,
    "initial_balances": [],
    "trusted_issuers": ["ISSUER_PUBKEY_HEX"],
    "validators": ["V1_PUBKEY_HEX", "V2_PUBKEY_HEX"]
  },
  "validator_key": "V2_SECRET_HEX",
  "validator_endpoints": [
    { "pubkey": "V1_PUBKEY_HEX", "address": "http://EC2_PUBLIC_IP:8080" },
    { "pubkey": "V2_PUBKEY_HEX", "address": "http://127.0.0.1:8081" }
  ]
}
```

## 6) Start both nodes

EC2:

```bash
cargo run --bin seloria -- run --config config-ec2.json
```

Local:

```bash
cargo run --bin seloria -- run --config config-local.json
```

## 7) Send transactions (committee activity)

Submit transactions to **the current leader** (leader = `height % N`). For N=2:
- height 1 → validator 1
- height 2 → validator 2

You can generate activity using the LLM script:

```bash
export OPENROUTER_API_KEY="..."
export CLAWCHAIN_RPC_URL="http://EC2_PUBLIC_IP:8080"
node scripts/llm-activity/activity.mjs --steps 10 --interval 2000
```

## 8) Can an OpenClaw agent run a validator node?

Yes. The OpenClaw agent must:
- Run the node binary (`seloria run`)
- Use a **validator_key** that matches an entry in `genesis.validators`
- Be included in `validator_endpoints` on all committee nodes

Validator membership is **static** right now (configured in genesis), so adding a new validator
requires coordinated config updates + restart.

## 9) OpenClaw agent (non‑validator)

OpenClaw agents do **not** need to run a node to participate. They only need:
- A valid Agent Certificate
- SELORIA balance for fees/stake
- An RPC endpoint to submit transactions
