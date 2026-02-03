"use client";

import { ExplorerHeader } from "@/components/explorer-header";
import { StatBox, TerminalBox } from "@/components/terminal-components";
import { Badge } from "@/components/ui/badge";
import { formatHash, formatTimeAgo } from "@/lib/format";
import { useRecentBlocks } from "@/lib/queries";
import { CaretRight } from "@phosphor-icons/react";
import Link from "next/link";

export default function BlocksPage() {
  const { data: blocks = [], isLoading: loading } = useRecentBlocks(20);

  if (loading) {
    return (
      <div className="min-h-screen">
        <ExplorerHeader />
        <main className="mx-auto max-w-7xl px-6 py-12">
          <div className="flex items-center justify-center py-20">
            <div className="text-primary border-2 border-dashed border-primary p-6 font-mono text-xs tracking-[0.3em]">
              [LOADING BLOCKS...]
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
        <div className="mb-8 border-2 border-dashed border-primary bg-card/50 p-6">
          <h1 className="text-primary mb-2 font-mono text-xl tracking-[0.2em]">
            BLOCKS
          </h1>
          <p className="text-muted-foreground font-mono text-xs tracking-wide">
            Browse all finalized blocks on the Seloria network
          </p>
        </div>

        <div className="mb-8 grid gap-6 md:grid-cols-3">
          <StatBox
            label="TOTAL BLOCKS"
            value={
              blocks.length > 0 ? (blocks[0].height + 1).toLocaleString() : "0"
            }
            sublabel="FINALIZED"
          />
          <StatBox
            label="LATEST BLOCK"
            value={blocks.length > 0 ? `#${blocks[0].height}` : "N/A"}
            sublabel="MOST RECENT"
          />
          <StatBox
            label="AVG TX/BLOCK"
            value={
              blocks.length > 0
                ? Math.round(
                    blocks.reduce((sum, b) => sum + b.tx_count, 0) /
                      blocks.length,
                  )
                : "0"
            }
            sublabel="AVERAGE"
          />
        </div>

        <TerminalBox>
          <div className="space-y-0">
            {blocks.length === 0 ? (
              <div className="text-muted-foreground py-12 text-center font-mono text-xs tracking-wider">
                [NO BLOCKS FOUND]
              </div>
            ) : (
              blocks.map((block) => (
                <Link
                  key={block.height}
                  href={`/block/${block.height}`}
                  className="group flex flex-col gap-3 border-b border-border/30 py-5 transition-colors hover:bg-primary/5 last:border-0 sm:flex-row sm:items-center sm:justify-between"
                >
                  <div className="flex items-start gap-4">
                    <div className="border border-primary bg-primary/10 px-3 py-2">
                      <span className="text-primary font-mono text-sm font-medium tracking-wider">
                        #{block.height}
                      </span>
                    </div>
                    <div className="flex-1 space-y-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="text-primary font-mono text-xs">
                          {formatHash(block.hash, 10)}
                        </span>
                        <Badge
                          variant="outline"
                          className="border-accent text-accent text-[10px]"
                        >
                          {block.tx_count} TX
                        </Badge>
                      </div>
                      <div className="text-muted-foreground space-y-1 font-mono text-[10px] tracking-wider">
                        <div>PROPOSER: {formatHash(block.proposer, 8)}</div>
                        <div>STATE ROOT: {formatHash(block.state_root, 8)}</div>
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center justify-between gap-4 sm:justify-end">
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
        </TerminalBox>
      </main>
    </div>
  );
}
