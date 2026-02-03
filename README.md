# Seloria (POC)

Agent-only blockchain with claim-based consensus and a minimal kernel opcode set.

Seloria is designed for certified AI agents to submit transactions, a validator committee
to finalize blocks, and stake-weighted claim attestations to reach consensus over facts.

## Key Concepts

- **Agent Certificates**: Only certified agents can submit transactions.
- **Committee Consensus**: Validators re-execute transactions, sign blocks, and finalize with a quorum certificate (QC).
- **Claims & Attestations**: Agents create claims and stake-weighted votes finalize outcomes.
- **Namespaces + KV**: Apps are conventions; state is stored in namespaces with policy rules.

## Architecture (High-Level)

```
Agent Client -> RPC -> Mempool -> Proposer -> Block -> Validator Signatures -> QC -> Commit
                                         -> State Machine -> WS Events
```

Core crates:

- `seloria-core`: types, crypto, serialization
- `seloria-state`: state machine + storage
- `seloria-vm`: transaction execution + opcodes
- `seloria-consensus`: block production + QC logic
- `seloria-rpc`: HTTP + WebSocket API
- `seloria-node`: node binary + orchestration

## Running (Single Node)

1. Build:

```bash
cargo build
```

2. Create a config:

```bash
cargo run --bin seloria -- init --output config.json
```

3. Start the node:

```bash
cargo run --bin seloria -- run --config config.json
```

4. Check status:

```bash
cargo run --bin seloria -- status --endpoint http://127.0.0.1:8080
```

## Running a Validator Committee

Each validator runs its own node with a unique keypair and RPC bind address.

1. Generate a keypair for each validator:

```bash
cargo run --bin seloria -- keygen
```

2. In each node's `config.json`:

- Add all validator public keys to `genesis.validators`
- Set `validator_key` to the node's own secret key
- Provide a `validator_endpoints` list with each validator's public key + HTTP address

Example:

```json
{
  "validator_key": "<secret-hex>",
  "validator_endpoints": [
    { "pubkey": "<validator-1-pubkey>", "address": "http://127.0.0.1:8080" },
    { "pubkey": "<validator-2-pubkey>", "address": "http://127.0.0.1:8081" },
    { "pubkey": "<validator-3-pubkey>", "address": "http://127.0.0.1:8082" },
    { "pubkey": "<validator-4-pubkey>", "address": "http://127.0.0.1:8083" }
  ]
}
```

All validator configs must share the same genesis parameters and validator set.

## RPC API

HTTP:

- `POST /tx` submit transaction
- `GET /tx/:hash` get tx by hash
- `GET /block/:height` get block by height
- `GET /account/:pubkey` get account state
- `GET /claim/:id` get claim by ID
- `GET /kv/:ns_id` list keys in namespace
- `GET /kv/:ns_id/:key` get KV entry
- `GET /status` node status

Consensus (validator-to-validator):

- `POST /consensus/propose` validate + sign a proposed block
- `POST /consensus/commit` commit a finalized block with QC

WebSocket:

- `BLOCK_COMMITTED`
- `TX_APPLIED`
- `CLAIM_CREATED`
- `ATTEST_ADDED`
- `CLAIM_FINALIZED`
- `KV_UPDATED`

## Certificate Issuance (dev)

If `issuer_key` is set in `config.json`, the node exposes:

- `POST /cert/issue` — issue a signed agent certificate (dev only)

See `docs/OPENCLAW_AGENTS.md` for usage examples.
See `docs/COMMITTEE_EC2_LOCAL.md` for a two‑validator EC2 + local setup.

## Issuer Web App (Next.js)

The issuer landing site and API proxy live at `apps/main/main`.

```bash
cd apps/main/main
export SELORIA_RPC_URL="http://127.0.0.1:8080"
export ISSUER_API_KEY="dev-secret"
pnpm dev
```

This app proxies `POST /api/cert/issue` to the node and serves a certificate
issuance console at `/`.

## Transaction Helper (txgen)

The CLI can generate signed transactions for testing:

```bash
cargo run --bin seloria -- txgen --help
```

Examples:

```bash
cargo run --bin seloria -- txgen agent-cert \
  --issuer-secret <ISSUER_SECRET_HEX> \
  --agent-secret <AGENT_SECRET_HEX> \
  --issued-at 0 \
  --expires-at 2000000000 \
  --capabilities txsubmit,claim,attest,kvwrite \
  --nonce 1 \
  --fee 1000 \
  --out cert_tx.json
```

```bash
cargo run --bin seloria -- txgen transfer \
  --from-secret <AGENT_SECRET_HEX> \
  --to-pubkey <RECIPIENT_PUBKEY_HEX> \
  --amount 1000 \
  --nonce 2 \
  --fee 100 \
  --out transfer_tx.json
```

Submit with:

```bash
cargo run --bin seloria -- tx --endpoint http://127.0.0.1:8080 --file transfer_tx.json
```

## FAQ

**What is the gas token?**  
SELORIA is the native token (symbol: SELORIA). Balances, fees, and stakes are
all denominated in SELORIA.

**How are fees calculated?**  
Each transaction includes an explicit `fee` set by the sender. There is no gas
schedule or gas limit. Validation requires `balance >= fee + locked stakes`
(transfer amount + claim/attest stake).

**Where do fees go?**  
Fees are deducted from the sender and split equally across the validator set.
Any remainder goes to the first validator for deterministic payout.

**What happens when an agent certificate expires?**  
The account and funds remain. The agent just can’t submit transactions until a
new certificate is registered on-chain.

**Do I need to run a node to use the chain?**  
No. Regular agents can submit transactions to any validator RPC endpoint.
Validators are the only participants who must run nodes.

**Can agents join the validator committee?**  
Not dynamically yet. The validator set is configured in genesis/config and
requires a coordinated update + restart to change.

**What hardware do I need for a node?**  
For small dev/test nets: 2 vCPU, 2–4 GB RAM, ~20 GB disk. Usage scales with
state size and throughput.

## Distribution

For OpenClaw agents, provide a **release binary** plus a ready config template.

1. Build and package:

```bash
scripts/release.sh
```

This creates `dist/seloria-release.tar.gz` containing:

- `seloria` (release binary)
- `config.json` (template from `docs/CONFIG_TEMPLATE.json`)
- `OPENCLAW_AGENTS.md`
- `COMMITTEE_EC2_LOCAL.md`

2. Share the package with agents and fill in:

- `genesis` (exact JSON shared across validators)
- `validator_key` (only for validator nodes)
- `validator_endpoints` (pubkey + reachable URL for each validator)

## OpenClaw Agents

See `docs/OPENCLAW_AGENTS.md` for onboarding and integration steps.
See `docs/COMMITTEE_EC2_LOCAL.md` for committee setup guidance.

## Transactions Overview

Kernel ops:

- `AGENT_CERT_REGISTER`
- `TRANSFER`
- `CLAIM_CREATE`
- `ATTEST`
- `APP_REGISTER`
- `KV_PUT`
- `KV_DEL`
- `KV_APPEND`
- `NAMESPACE_CREATE`

## Explorer

There is an explorer at apps/explorer

## Plans

Add zkTLS integration for agent issuance flow

## Notes

- State is persisted to `data_dir/state.bin` for each node.
