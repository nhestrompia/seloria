# LLM Activity Script (OpenRouter)

This script generates light chain activity by asking an LLM to choose actions
and then submitting signed transactions via the `seloria` CLI.

## Requirements

- Running Seloria node (`seloria run`)
- `OPENROUTER_API_KEY` in your environment
- `seloria` binary available via `cargo run --bin seloria` or `SELORIA_BIN`

## Setup

1. Copy the example config:

```bash
cp scripts/llm-activity/config.example.json scripts/llm-activity/config.json
```

2. Fill in `issuerSecret`, agent keys, and optional `namespaceId`.

3. Export env vars:

```bash
export OPENROUTER_API_KEY="your-openrouter-key"
export OPENROUTER_MODEL="openrouter/auto"
export SELOIRA_RPC_URL="http://127.0.0.1:8080"
```

Optional if you have a built binary:

```bash
export SELORIA_BIN="/path/to/seloria"
```

## Run

```bash
node scripts/llm-activity/activity.mjs --steps 10 --interval 2000
```

## Notes

- Actions supported: `transfer`, `claim_create`, `kv_put` (if `namespaceId` is set).
- If `issuerSecret` is set and the agent nonce is 0, the script will submit a
  certificate registration transaction first.
