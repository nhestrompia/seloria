"use client";

import { ExplorerHeader } from "@/components/explorer-header";
import { DataRow, TerminalBox } from "@/components/terminal-components";
import { Badge } from "@/components/ui/badge";
import { formatBalance, formatHash } from "@/lib/format";
import { useAccount } from "@/lib/queries";
import { CaretLeft } from "@phosphor-icons/react";
import Link from "next/link";
import { useParams } from "next/navigation";

export default function AccountDetailPage() {
  const params = useParams();
  const pubkey = params.pubkey as string;

  const { data: account, isLoading: loading, error } = useAccount(pubkey);

  if (loading) {
    return (
      <div className="min-h-screen">
        <ExplorerHeader />
        <main className="mx-auto max-w-7xl px-6 py-12">
          <div className="flex items-center justify-center py-20">
            <div className="text-primary border-2 border-dashed border-primary p-6 font-mono text-xs tracking-[0.3em]">
              [LOADING ACCOUNT...]
            </div>
          </div>
        </main>
      </div>
    );
  }

  if (error || (!loading && !account)) {
    return (
      <div className="min-h-screen">
        <ExplorerHeader />
        <main className="mx-auto max-w-7xl px-6 py-12">
          <div className="border-2 border-dashed border-destructive bg-destructive/10 p-8 text-center">
            <div className="text-destructive mb-4 font-mono text-xl tracking-[0.3em]">
              [ERROR]
            </div>
            <p className="text-muted-foreground font-mono text-xs tracking-wider">
              {error?.message || "Account not found"}
            </p>
            <Link
              href="/accounts"
              className="text-primary hover:text-accent mt-6 inline-flex items-center gap-2 font-mono text-xs tracking-wider transition-colors"
            >
              <CaretLeft weight="bold" size={14} />
              BACK TO ACCOUNTS
            </Link>
          </div>
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen">
      <ExplorerHeader />

      <main className="mx-auto max-w-7xl px-6 py-12">
        <div className="mb-6">
          <Link
            href="/accounts"
            className="text-muted-foreground hover:text-primary inline-flex items-center gap-2 font-mono text-xs tracking-wider transition-colors"
          >
            <CaretLeft weight="bold" size={14} />
            BACK TO ACCOUNTS
          </Link>
        </div>

        <div className="mb-8 border-2 border-dashed border-primary bg-card/50 p-6">
          <h1 className="text-primary mb-2 font-mono text-xl tracking-[0.2em]">
            ACCOUNT
          </h1>
          <p className="text-muted-foreground font-mono text-xs tracking-wide">
            Agent account details and state
          </p>
        </div>

        <div className="mb-8 grid gap-6 md:grid-cols-2">
          <TerminalBox title="ACCOUNT STATE">
            <DataRow
              label="BALANCE"
              value={formatBalance(account?.balance ?? 0)}
            />
            <DataRow label="NONCE" value={account?.nonce} />
            <DataRow
              label="TOTAL BALANCE"
              value={formatBalance(account?.total_balance ?? 0)}
            />
            <DataRow
              label="STATUS"
              value={
                <Badge
                  variant="outline"
                  className="border-primary text-primary"
                >
                  ACTIVE
                </Badge>
              }
              mono={false}
            />
          </TerminalBox>

          <TerminalBox title="IDENTITY">
            <DataRow
              label="PUBLIC KEY"
              value={formatHash(account?.pubkey ?? "", 12)}
            />
            <DataRow
              label="CERTIFIED"
              value={
                <Badge variant="outline" className="border-accent text-accent">
                  AGENT
                </Badge>
              }
              mono={false}
            />
          </TerminalBox>
        </div>

        <TerminalBox title="TRANSACTION HISTORY" subtitle="Coming soon">
          <div className="text-muted-foreground py-12 text-center font-mono text-xs tracking-wider">
            [TRANSACTION HISTORY COMING SOON]
          </div>
        </TerminalBox>

        <div className="mt-6 border-t border-border/50 pt-6">
          <div className="text-muted-foreground font-mono text-[10px] tracking-wider">
            <div>FULL PUBLIC KEY: {account?.pubkey}</div>
          </div>
        </div>
      </main>
    </div>
  );
}
