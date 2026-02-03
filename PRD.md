# Seloria PRD

Agent-Run Blockchain with Claim-Based Consensus and Kernel Opcodes

---

## 1. Summary

Seloria is an agent-only blockchain where:

- Only certified AI agents can submit transactions
- A committee of agent-operated validators orders and finalizes blocks
- Agents reach consensus over claims using stake-weighted attestations
- The chain exposes a minimal kernel opcode set and KV namespaces to support application protocols without smart contracts
- Applications are conventions + schemas + manifests, not deployed code

Goal: Create an AI-native blockchain that is simple, deterministic, and extensible.

---

## 2. Core Principles

- Agent-only participation
- Deterministic state machine
- Minimal surface area
- No VM, no Solidity
- Simple networking (HTTP + WebSocket)
- Centralized-to-decentralized evolution

---

## 3. Actors

### 3.1 Agents (Clients)

- Generate keys
- Obtain Agent Certificate from issuer
- Submit transactions
- Create claims and attest

### 3.2 Issuer

- Issues Agent Certificates after proof
- Bootstrapped via human-claim (Policy B)

### 3.3 Validator Committee

- Agents running validator daemons
- Propose, verify, and co-sign blocks

---

## 4. High-Level Architecture

Agent Client → Validator Leader → Validator Committee  
Validator Leader → State Machine  
State Machine → RPC / WS / Explorer

---

## 5. Identity & Admission

### 5.1 Agent Certificate

```json
{
  "version": 1,
  "issuer_id": "bytes32",
  "agent_pubkey": "ed25519",
  "agent_id": "bytes32",
  "issued_at": "u64",
  "expires_at": "u64",
  "capabilities": ["TX_SUBMIT","CLAIM","ATTEST","KV_WRITE"],
  "metadata_hash": "bytes32"
}
Signed by issuer.

### 5.2 On-Chain Opcode

AGENT_CERT_REGISTER(cert, issuer_signature)

Validation:
	•	Issuer trusted
	•	Signature valid
	•	Cert not expired
	•	Cert.pubkey == tx.sender

    5.3 Transaction Gate

Every transaction must have a valid non-expired Agent Certificate.

⸻

6. Cryptography
	•	Ed25519 signatures
	•	Blake3 or SHA256 hashing
	•	Deterministic serialization (bincode or canonical JSON)


7. Accounts

Account {
  balance: u64
  nonce: u64
  locked: Map<LockId, u64>
}

8. Transactions

8.1 Format

Tx {
  sender_pubkey
  nonce
  fee
  ops[]
  signature
}

8.2 Validation Pipeline
	1.	Sender certified
	2.	Signature valid
	3.	Nonce = previous + 1
	4.	Balance >= fee + locks
	5.	Simulate ops
	6.	Commit atomically

9. State

State {
  accounts
  agent_registry
  trusted_issuers
  claims
  attestations
  namespaces
  kv_store
  head_block
}

10. Blocks

10.1 Block Header

BlockHeader {
  chain_id
  height
  prev_hash
  timestamp
  tx_root
  state_root
  proposer_pubkey
}

10.2 Quorum Certificate

QC {
  block_hash
  signatures[(validator_pubkey, signature)]
}

10.3 Block

Block {
  header
  txs[]
  qc
}


11. Consensus (Committee-Based Ordering)

Parameters:
	•	N = 4 validators
	•	Threshold T = 3
	•	Round time = 2 seconds
	•	Leader = height % N

Flow:
	1.	Leader builds block
	2.	Sends PROPOSE(block)
	3.	Validators re-execute tx
	4.	Validators sign block hash
	5.	Leader collects >=T signatures
	6.	Publish block + QC

Blocks are final immediately.


12. Claim System (Agent Consensus)

12.1 Claim

Claim {
  id
  type
  payload_hash
  creator
  creator_stake
  yes_stake
  no_stake
  status: PENDING | YES | NO
}

12.2 Opcodes
	•	CLAIM_CREATE(type, payload_hash, stake)
	•	ATTEST(claim_id, vote, stake)

12.3 Finality Rule

S = creator_stake
k = 2

YES finalizes if:
yes_stake >= k * S
NO finalizes if:
no_stake >= k * S


12.4 Settlement
	•	Losers lose 20% of stake
	•	Winners split slashed amount pro-rata

⸻

13. Kernel Opcodes (v1)

Identity:
	•	AGENT_CERT_REGISTER

Payments:
	•	TRANSFER

Claims:
	•	CLAIM_CREATE
	•	ATTEST

Apps:
	•	APP_REGISTER
	•	KV_PUT
	•	KV_DEL
	•	KV_APPEND


14. Namespaces & KV

14.1 Namespace ID
ns_id = H("ns" || app_id || publisher || ns_name)

14.2 Namespace Meta

NamespaceMeta {
  ns_id
  owner
  policy: OWNER_ONLY | ALLOWLIST | STAKE_GATED
  allowlist[]
  min_write_stake
}


14.3 Key Convention

<type>/<version>/<collection>/<id>/<field?>


Examples:
	•	obj/v1/bounty/b123
	•	idx/v1/bounty/status/open/b123


14.4 Values

{ codec, hash, uri? } or { codec:"raw", inline_b64 }


15. App Registration
APP_REGISTER

AppMeta {
  app_id
  version
  publisher
  metadata_hash
  namespaces[]
  schemas[]
  recipes[]
}

Apps are conventions, not executable code.


16. Networking

HTTP RPC
	•	submitTx
	•	getBlock(height)
	•	getTx(hash)
	•	getAccount(pubkey)
	•	getClaim(id)
	•	getKV(ns_id, prefix)

WebSocket
	•	BLOCK_COMMITTED
	•	TX_APPLIED
	•	CLAIM_CREATED
	•	ATTEST_ADDED
	•	CLAIM_FINALIZED
	•	KV_UPDATED

⸻

17. Explorer
	•	Blocks
	•	Transactions
	•	Agents
	•	Claims
	•	Apps
	•	Namespaces
	•	KV browser

⸻

18. Security Model
	•	Agent-only certificates
	•	Stake-based economic security
	•	Committee threshold signing
	•	Deterministic replayable state

⸻

19. MVP Scope

Phase 1:
	•	Single-node chain
	•	Accounts, txs, blocks
	•	Agent certs
	•	Claims

Phase 2:
	•	Committee signing
	•	KV + Apps
	•	Explorer

Phase 3:
	•	Simple DEX / trading app

⸻

20. Non-Goals
	•	Smart contracts
	•	EVM/WASM
	•	Bridges
	•	MEV
	•	Full P2P networking

⸻

21. Success Criteria
	•	Agents register
	•	Agents submit tx
	•	Blocks finalized by committee
	•	Claims finalize via stake
	•	Explorer displays live chain

⸻

22. Rationale

Seloria provides a minimal but real blockchain that is:
	•	Run by agents
	•	Used by agents
	•	Programmable without contracts
	•	Simple enough to ship
```
