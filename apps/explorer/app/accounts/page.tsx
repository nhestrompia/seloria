"use client";

import { ExplorerHeader } from "@/components/explorer-header";
import { TerminalBox } from "@/components/terminal-components";
import { MagnifyingGlass } from "@phosphor-icons/react";
import { useRouter } from "next/navigation";
import { useState } from "react";

export default function AccountsPage() {
  const router = useRouter();
  const [pubkey, setPubkey] = useState("");

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (pubkey.trim()) {
      router.push(`/account/${pubkey.trim()}`);
    }
  };

  return (
    <div className="min-h-screen">
      <ExplorerHeader />

      <main className="mx-auto max-w-7xl px-6 py-12">
        <div className="mb-8 border-2 border-dashed border-primary bg-card/50 p-6">
          <h1 className="text-primary mb-2 font-mono text-xl tracking-[0.2em]">
            ACCOUNTS
          </h1>
          <p className="text-muted-foreground font-mono text-xs tracking-wide">
            Search and view agent accounts on Seloria
          </p>
        </div>

        <TerminalBox title="ACCOUNT LOOKUP">
          <form onSubmit={handleSearch} className="space-y-4">
            <div>
              <label className="text-muted-foreground mb-2 block font-mono text-xs tracking-wider">
                PUBLIC KEY
              </label>
              <div className="flex gap-3">
                <input
                  type="text"
                  value={pubkey}
                  onChange={(e) => setPubkey(e.target.value)}
                  placeholder="Enter agent public key..."
                  className="border-border bg-input text-foreground placeholder:text-muted-foreground flex-1 border px-4 py-3 font-mono text-xs tracking-wide outline-none ring-primary focus:border-primary focus:ring-1"
                />
                <button
                  type="submit"
                  className="bg-primary text-primary-foreground hover:bg-primary/90 flex items-center gap-2 px-6 py-3 font-mono text-xs tracking-wider transition-colors"
                >
                  <MagnifyingGlass weight="bold" size={14} />
                  SEARCH
                </button>
              </div>
            </div>
          </form>

          <div className="text-muted-foreground mt-8 border-t border-border/50 pt-6 font-mono text-[10px] tracking-wider">
            <div className="mb-2">INSTRUCTIONS:</div>
            <ul className="space-y-1 pl-4">
              <li>
                • Enter the agent's Ed25519 public key (64 hex characters)
              </li>
              <li>• View account balance, nonce, and transaction history</li>
              <li>• Only certified agents can have accounts</li>
            </ul>
          </div>
        </TerminalBox>

        <div className="mt-8">
          <TerminalBox title="REGISTERED AGENTS" subtitle="Coming soon">
            <div className="text-muted-foreground py-12 text-center font-mono text-xs tracking-wider">
              [AGENT REGISTRY COMING SOON]
            </div>
          </TerminalBox>
        </div>
      </main>
    </div>
  );
}
