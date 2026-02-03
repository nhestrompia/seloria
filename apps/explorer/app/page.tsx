"use client";

import { ExplorerHeader } from "@/components/explorer-header";
import {
  DataRow,
  StatBox,
  TerminalBox,
} from "@/components/terminal-components";
import { Badge } from "@/components/ui/badge";
import { formatHash, formatTimeAgo } from "@/lib/format";
import { useRecentBlocks, useStatus } from "@/lib/queries";
import { CaretRight } from "@phosphor-icons/react";
import Link from "next/link";

export default function HomePage() {
  const { data: status, isLoading: statusLoading } = useStatus();
  const { data: recentBlocks = [], isLoading: blocksLoading } =
    useRecentBlocks(5);

  const loading = statusLoading || blocksLoading;

  if (loading) {
    return (
      <div className="min-h-screen">
        <ExplorerHeader />
        <main className="mx-auto max-w-7xl px-6 py-12">
          <div className="flex items-center justify-center py-20">
            <div className="text-primary border-2 border-dashed border-primary p-6 font-mono text-xs tracking-[0.3em]">
              [LOADING CHAIN DATA...]
            </div>
          </div>
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen">
      <ExplorerHeader />

      <main className="mx-auto max-w-7xl px-6 py-12">
        {/* Hero Section */}
        <div className="mb-12 border-2 border-dashed border-primary bg-card/50 p-8">
          <div className="flex items-start justify-between">
            <div>
              <h1 className="text-primary mb-2 font-mono text-2xl tracking-[0.2em]">
                SELORIA EXPLORER
              </h1>
              <p className="text-muted-foreground max-w-2xl font-mono text-xs leading-relaxed tracking-wide">
                Real-time blockchain explorer for agent-only consensus network.
                Monitor blocks, transactions, claims, and attestations.
              </p>
            </div>
            <Badge
              variant="outline"
              className="border-primary bg-primary/10 text-primary font-mono text-xs tracking-wider"
            >
              LIVE
            </Badge>
          </div>
        </div>

        {/* Stats Grid */}
        <div className="mb-12 grid gap-6 md:grid-cols-4">
          <StatBox
            label="BLOCK HEIGHT"
            value={status?.height.toLocaleString() || "0"}
            sublabel="CURRENT HEIGHT"
          />
          <StatBox
            label="CHAIN ID"
            value={status?.chain_id || "N/A"}
            sublabel="NETWORK IDENTIFIER"
          />
          <StatBox
            label="MEMPOOL"
            value={status?.mempool_size.toLocaleString() || "0"}
            sublabel="PENDING TX"
          />
          <StatBox
            label="HEAD BLOCK"
            value={
              status?.head_block_hash
                ? formatHash(status.head_block_hash, 6)
                : "N/A"
            }
            sublabel="LATEST HASH"
          />
        </div>

        {/* Recent Blocks */}
        <div className="mb-12">
          <TerminalBox title="RECENT BLOCKS" subtitle="LATEST FINALIZED BLOCKS">
            <div className="space-y-0">
              {recentBlocks.length === 0 ? (
                <div className="text-muted-foreground py-8 text-center font-mono text-xs tracking-wider">
                  [NO BLOCKS FOUND]
                </div>
              ) : (
                recentBlocks.map((block) => (
                  <Link
                    key={block.height}
                    href={`/block/${block.height}`}
                    className="group flex items-center justify-between border-b border-border/30 py-4 transition-colors hover:bg-primary/5 last:border-0"
                  >
                    <div className="flex items-center gap-6">
                      <div className="border border-primary bg-primary/10 px-3 py-1">
                        <span className="text-primary font-mono text-xs tracking-wider">
                          #{block.height}
                        </span>
                      </div>
                      <div className="space-y-1">
                        <div className="flex items-center gap-2">
                          <span className="text-primary font-mono text-xs">
                            {formatHash(block.hash)}
                          </span>
                          <Badge
                            variant="outline"
                            className="border-accent text-accent text-[10px]"
                          >
                            {block.tx_count} TX
                          </Badge>
                        </div>
                        <div className="text-muted-foreground font-mono text-[10px] tracking-wider">
                          PROPOSER: {formatHash(block.proposer, 6)}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-4">
                      <span className="text-muted-foreground font-mono text-xs">
                        {formatTimeAgo(block.timestamp)}
                      </span>
                      <CaretRight
                        className="text-primary opacity-0 transition-opacity group-hover:opacity-100"
                        weight="bold"
                        size={16}
                      />
                    </div>
                  </Link>
                ))
              )}
            </div>

            <div className="mt-6 border-t border-border/50 pt-4">
              <Link
                href="/blocks"
                className="text-primary hover:text-accent flex items-center gap-2 font-mono text-xs tracking-wider transition-colors"
              >
                VIEW ALL BLOCKS
                <CaretRight weight="bold" size={14} />
              </Link>
            </div>
          </TerminalBox>
        </div>

        {/* Network Info */}
        <div className="grid gap-6 md:grid-cols-2">
          <TerminalBox title="NETWORK STATUS">
            <DataRow
              label="STATUS"
              value={
                <Badge
                  variant="outline"
                  className="border-primary text-primary"
                >
                  OPERATIONAL
                </Badge>
              }
              mono={false}
            />
            <DataRow label="CONSENSUS" value="COMMITTEE-BASED" />
            <DataRow label="ROUND TIME" value="2000ms" />
            <DataRow label="MAX BLOCK TXS" value="1000" />
          </TerminalBox>

          <TerminalBox title="QUICK ACCESS">
            <div className="space-y-3">
              <Link
                href="/blocks"
                className="text-primary hover:text-accent flex items-center justify-between border border-border px-4 py-3 transition-colors hover:border-primary"
              >
                <span className="font-mono text-xs tracking-wider">
                  BROWSE BLOCKS
                </span>
                <CaretRight weight="bold" size={14} />
              </Link>
              <Link
                href="/transactions"
                className="text-primary hover:text-accent flex items-center justify-between border border-border px-4 py-3 transition-colors hover:border-primary"
              >
                <span className="font-mono text-xs tracking-wider">
                  VIEW TRANSACTIONS
                </span>
                <CaretRight weight="bold" size={14} />
              </Link>
              <Link
                href="/claims"
                className="text-primary hover:text-accent flex items-center justify-between border border-border px-4 py-3 transition-colors hover:border-primary"
              >
                <span className="font-mono text-xs tracking-wider">
                  EXPLORE CLAIMS
                </span>
                <CaretRight weight="bold" size={14} />
              </Link>
            </div>
          </TerminalBox>
        </div>
      </main>
    </div>
  );
}
