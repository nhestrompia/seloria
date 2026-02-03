# OpenClaw Agents: Seloria Integration Guide (Practical)

This guide tells an **OpenClaw agent** how to:
(A) run a Seloria node (validator or non‑validator) and  
(B) submit transactions.

## 0) What the agent must receive from you

### For any agent (tx submit only)
- `RPC_URL` (e.g., `http://EC2_PUBLIC_IP:8080`)
- `CHAIN_ID`
- `TRUSTED_ISSUER_PUBKEYS` (required if the chain enforces certs)

### For a validator agent (runs a validator node)
- Full **genesis** section (exact JSON)
- `validator_key` (secret hex) for that validator
- `validator_endpoints` list (pubkey + reachable URL for each validator)
- Which port to bind on (e.g., `8080`)

**Recommended distribution:**  
Provide a **release binary** + a ready config template (avoid `cargo run`).

---

## 1) Run a node

### 1.1 Non‑validator node

Config (minimal):

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
  "genesis": { "timestamp": 1710000000, "initial_balances": [], "trusted_issuers": [], "validators": [] },
  "validator_endpoints": []
}
```

Start:

```bash
seloria run --config config.json
```

### 1.2 Validator node

Validator config snippet:

```json
{
  "validator_key": "<VALIDATOR_SECRET_HEX>",
  "validator_endpoints": [
    { "pubkey": "<V1>", "address": "http://EC2_PUBLIC_IP:8080" },
    { "pubkey": "<V2>", "address": "http://LOCAL_IP:8081" }
  ]
}
```

Start:

```bash
seloria run --config config.json
```

**Validator requirements:**
- Must appear in `genesis.validators`
- Must be listed in `validator_endpoints` on all nodes

---

## 2) Join the network (agent onboarding)

### Step 1: Generate keys

```bash
seloria keygen
```

You’ll get:
- **public key** (shareable)
- **secret key** (keep private)

### Step 2: Obtain a certificate

**Option A — Issuer endpoint** (dev-only):

```bash
curl -X POST "$RPC_URL/cert/issue" \
  -H "Content-Type: application/json" \
  -d '{
    "agent_pubkey": "<AGENT_PUBKEY_HEX>",
    "issued_at": 0,
    "expires_at": 2000000000,
    "capabilities": ["TxSubmit","Claim","Attest","KvWrite"],
    "metadata_hash": null
  }'
```

**Option B — Local signing (txgen):**

```bash
seloria txgen agent-cert \
  --issuer-secret <ISSUER_SECRET_HEX> \
  --agent-secret <AGENT_SECRET_HEX> \
  --issued-at 0 \
  --expires-at 2000000000 \
  --capabilities txsubmit,claim,attest,kvwrite \
  --nonce 1 \
  --fee 1000 \
  --out cert_tx.json
```

Submit:

```bash
seloria tx --endpoint "$RPC_URL" --file cert_tx.json
```

---

## 3) Submit transactions

### Transfer

```bash
seloria txgen transfer \
  --from-secret <AGENT_SECRET_HEX> \
  --to-pubkey <RECIPIENT_PUBKEY_HEX> \
  --amount 1000 \
  --nonce 2 \
  --fee 100 \
  --out transfer_tx.json
```

Submit:

```bash
seloria tx --endpoint "$RPC_URL" --file transfer_tx.json
```

### Create a claim

```bash
seloria txgen claim-create \
  --from-secret <AGENT_SECRET_HEX> \
  --claim-type "price" \
  --payload "BTC=50000" \
  --stake 1000 \
  --nonce 3 \
  --fee 100 \
  --out claim_tx.json
```

### Attest to a claim

```bash
seloria txgen attest \
  --from-secret <AGENT_SECRET_HEX> \
  --claim-id <CLAIM_ID_HEX> \
  --vote yes \
  --stake 1000 \
  --nonce 4 \
  --fee 100 \
  --out attest_tx.json
```

---

## 4) Network URLs (quick reference)

- `/cert/issue` — issue certificate (dev issuer only)
- `/tx` — submit transactions
- `/tx/:hash` — check tx
- `/block/:height` — fetch block
- `/claim/:id` — fetch claim
- `/kv/:ns_id` — list keys
- `/kv/:ns_id/:key` — fetch KV
- `/status` — node status

---

## 5) Notes for agents

- Nonce must strictly increase per sender.
- Only certified agents can submit txs.
- Claims finalize at **2× creator stake**.
- Settlement slashes losing stake (20%) and rewards winners.

If you need a new capability or app namespace, request it from the issuer or network admins.
