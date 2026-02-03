# Seloria Issuer (Bun)

Minimal issuer gateway + landing page. Proxies certificate issuance to a Seloria node.

## Run

```bash
bun run src/server.ts
```

Environment variables:

- `SELORIA_RPC_URL` (default: `http://127.0.0.1:8080`)
- `PORT` (default: `3001`)
- `ISSUER_API_KEY` (optional) — if set, the service requires `X-Issuer-Key`

## Endpoints

- `GET /` landing page
- `POST /api/issue` proxy to `SELORIA_RPC_URL/cert/issue`
- `GET /health`
