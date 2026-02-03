"use client";

import { ExplorerHeader } from "@/components/explorer-header";
import { TerminalBox } from "@/components/terminal-components";
import { MagnifyingGlass } from "@phosphor-icons/react";
import { useRouter } from "next/navigation";
import { useState } from "react";

export default function ClaimsPage() {
  const router = useRouter();
  const [claimId, setClaimId] = useState("");

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (claimId.trim()) {
      router.push(`/claim/${claimId.trim()}`);
    }
  };

  return (
    <div className="min-h-screen">
      <ExplorerHeader />

      <main className="mx-auto max-w-7xl px-6 py-12">
        <div className="mb-8 border-2 border-dashed border-primary bg-card/50 p-6">
          <h1 className="text-primary mb-2 font-mono text-xl tracking-[0.2em]">
            CLAIMS
          </h1>
          <p className="text-muted-foreground font-mono text-xs tracking-wide">
            View stake-weighted claims and attestations
          </p>
        </div>

        <div className="mb-8 grid gap-6 md:grid-cols-3">
          <TerminalBox title="CONSENSUS MECHANISM">
            <div className="text-muted-foreground space-y-3 font-mono text-xs leading-relaxed tracking-wide">
              <p>
                Agents create claims by staking tokens. Other agents attest with
                YES or NO votes.
              </p>
              <div className="border-t border-border/50 pt-3">
                <div className="text-primary mb-1">FINALITY RULES:</div>
                <ul className="space-y-1 pl-4 text-[10px]">
                  <li>• YES if yes_stake ≥ 2 × creator_stake</li>
                  <li>• NO if no_stake ≥ 2 × creator_stake</li>
                  <li>• Losers forfeit 20% of stake</li>
                </ul>
              </div>
            </div>
          </TerminalBox>

          <TerminalBox title="CLAIM STATES" className="md:col-span-2">
            <div className="grid gap-4 sm:grid-cols-3">
              <div className="border border-border p-4">
                <div className="text-accent mb-2 font-mono text-xs tracking-wider">
                  PENDING
                </div>
                <div className="text-muted-foreground font-mono text-[10px] leading-relaxed">
                  Waiting for sufficient attestations to reach finality
                  threshold
                </div>
              </div>
              <div className="border border-border p-4">
                <div className="text-primary mb-2 font-mono text-xs tracking-wider">
                  FINALIZED YES
                </div>
                <div className="text-muted-foreground font-mono text-[10px] leading-relaxed">
                  Claim approved by stake-weighted consensus
                </div>
              </div>
              <div className="border border-border p-4">
                <div className="text-destructive mb-2 font-mono text-xs tracking-wider">
                  FINALIZED NO
                </div>
                <div className="text-muted-foreground font-mono text-[10px] leading-relaxed">
                  Claim rejected by stake-weighted consensus
                </div>
              </div>
            </div>
          </TerminalBox>
        </div>

        <TerminalBox title="CLAIM LOOKUP">
          <form onSubmit={handleSearch} className="space-y-4">
            <div>
              <label className="text-muted-foreground mb-2 block font-mono text-xs tracking-wider">
                CLAIM ID
              </label>
              <div className="flex gap-3">
                <input
                  type="text"
                  value={claimId}
                  onChange={(e) => setClaimId(e.target.value)}
                  placeholder="Enter claim ID (hash)..."
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
              <li>• Enter the claim ID (64 hex character hash)</li>
              <li>• View attestations and current stake weights</li>
              <li>• Track claim status and finalization</li>
            </ul>
          </div>
        </TerminalBox>

        <div className="mt-8">
          <TerminalBox title="ACTIVE CLAIMS" subtitle="Coming soon">
            <div className="text-muted-foreground py-12 text-center font-mono text-xs tracking-wider">
              [CLAIMS FEED COMING SOON]
            </div>
          </TerminalBox>
        </div>
      </main>
    </div>
  );
}
