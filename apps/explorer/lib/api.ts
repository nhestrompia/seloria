// Seloria RPC API Client

const API_BASE_URL = process.env.NEXT_PUBLIC_RPC_URL || "http://localhost:8080";

export interface StatusResponse {
  chain_id: number;
  height: number;
  head_block_hash: string | null;
  mempool_size: number;
}

export interface BlockResponse {
  height: number;
  hash: string;
  prev_hash: string;
  timestamp: number;
  tx_count: number;
  proposer: string;
  tx_root: string;
  state_root: string;
}

export interface Transaction {
  sender_pubkey: string;
  nonce: number;
  fee: number;
  ops: any[];
  signature: string;
}

export interface AccountResponse {
  pubkey: string;
  balance: number;
  nonce: number;
  total_balance: number;
}

export interface ClaimResponse {
  id: string;
  claim_type: string;
  payload_hash: string;
  creator: string;
  creator_stake: number;
  yes_stake: number;
  no_stake: number;
  status: string;
  created_at: number;
  attestation_count: number;
}

class SeloriaAPI {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  async getStatus(): Promise<StatusResponse> {
    const res = await fetch(`${this.baseUrl}/status`);
    if (!res.ok) throw new Error("Failed to fetch status");
    return res.json();
  }

  async getBlock(height: number): Promise<BlockResponse> {
    const res = await fetch(`${this.baseUrl}/block/${height}`);
    if (!res.ok) throw new Error(`Failed to fetch block ${height}`);
    return res.json();
  }

  async getRecentBlocks(limit: number = 10): Promise<BlockResponse[]> {
    try {
      const status = await this.getStatus();
      const promises = [];
      const start = Math.max(0, status.height - limit + 1);

      for (let i = status.height; i >= start && i >= 0; i--) {
        promises.push(this.getBlock(i).catch(() => null));
      }

      const blocks = await Promise.all(promises);
      return blocks.filter((b): b is BlockResponse => b !== null);
    } catch {
      return [];
    }
  }

  async getTransaction(hash: string): Promise<Transaction> {
    const res = await fetch(`${this.baseUrl}/tx/${hash}`);
    if (!res.ok) throw new Error(`Failed to fetch transaction ${hash}`);
    return res.json();
  }

  async getAccount(pubkey: string): Promise<AccountResponse> {
    const res = await fetch(`${this.baseUrl}/account/${pubkey}`);
    if (!res.ok) throw new Error(`Failed to fetch account ${pubkey}`);
    return res.json();
  }

  async getClaim(id: string): Promise<ClaimResponse> {
    const res = await fetch(`${this.baseUrl}/claim/${id}`);
    if (!res.ok) throw new Error(`Failed to fetch claim ${id}`);
    return res.json();
  }
}

export const api = new SeloriaAPI();
