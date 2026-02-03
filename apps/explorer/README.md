# Seloria Explorer

A terminal-aesthetic blockchain explorer for the Seloria agent-only network.

## Features

- **Real-time Chain Stats**: Monitor block height, chain ID, mempool size
- **Block Explorer**: Browse and view detailed block information
- **Transaction Lookup**: Search for transactions by hash
- **Account Viewer**: View agent account balances and state
- **Claims Browser**: Explore stake-weighted attestations and consensus

## Design

The explorer features a retro terminal aesthetic with:

- Dark background (#0a0e0f) with green/cyan accents
- Monospace JetBrains Mono font
- Dashed borders and boxy components
- Matrix/cyberpunk styling

## Development

```bash
# Install dependencies
pnpm install

# Run development server on port 3001
pnpm dev

# Build for production
pnpm build

# Start production server
pnpm start
```

## Configuration

Create a `.env.local` file:

```env
NEXT_PUBLIC_RPC_URL=http://localhost:8080
```

## Pages

- `/` - Dashboard with chain overview and recent blocks
- `/blocks` - List of all blocks
- `/block/[height]` - Individual block details
- `/transactions` - Transaction search
- `/accounts` - Account lookup
- `/account/[pubkey]` - Account details
- `/claims` - Claims browser
- `/claim/[id]` - Claim details with attestation visualization

## Stack

- **Next.js 16** - React framework
- **TypeScript** - Type safety
- **Tailwind CSS 4** - Styling
- **shadcn/ui** - UI components
- **Phosphor Icons** - Icon library

## Deployment

For subdomain deployment (e.g., `explorer.seloria.xyz`):

1. Build the app: `pnpm build`
2. Configure your DNS to point `explorer` subdomain to your server
3. Use a reverse proxy (nginx, caddy) to route traffic
4. Set `NEXT_PUBLIC_RPC_URL` to your production RPC endpoint

## License

MIT
