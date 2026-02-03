"use client";

import { ExplorerHeader } from "@/components/explorer-header";
import { DataRow, TerminalBox } from "@/components/terminal-components";
import { Badge } from "@/components/ui/badge";
import { formatHash, formatTimestamp } from "@/lib/format";
import { useBlock } from "@/lib/queries";
import { CaretLeft, CaretRight } from "@phosphor-icons/react";
import Link from "next/link";
import { useParams } from "next/navigation";

export default function BlockDetailPage() {
  const params = useParams();
  const height = parseInt(params.height as string);

  const { data: block, isLoading: loading, error } = useBlock(height);

  if (loading) {
    return (
      <div className="min-h-screen">
        <ExplorerHeader />
        <main className="mx-auto max-w-7xl px-6 py-12">
          <div className="flex items-center justify-center py-20">
            <div className="text-primary border-2 border-dashed border-primary p-6 font-mono text-xs tracking-[0.3em]">
              [LOADING BLOCK...]
            </div>
          </div>
        </main>
      </div>
    );
  }

  if (error || (!loading && !block)) {
    return (
      <div className="min-h-screen">
        <ExplorerHeader />
        <main className="mx-auto max-w-7xl px-6 py-12">
          <div className="border-2 border-dashed border-destructive bg-destructive/10 p-8 text-center">
            <div className="text-destructive mb-4 font-mono text-xl tracking-[0.3em]">
              [ERROR]
            </div>
            <p className="text-muted-foreground font-mono text-xs tracking-wider">
              {error?.message || "Block not found"}
            </p>
            <Link
              href="/blocks"
              className="text-primary hover:text-accent mt-6 inline-flex items-center gap-2 font-mono text-xs tracking-wider transition-colors"
            >
              <CaretLeft weight="bold" size={14} />
              BACK TO BLOCKS
            </Link>
          </div>
        </main>
      </div>
    );
  }

  if (!block) {
    return null;
  }

  return (
    <div className="min-h-screen">
      <ExplorerHeader />

      <main className="mx-auto max-w-7xl px-6 py-12">
        <div className="mb-6">
          <Link
            href="/blocks"
            className="text-muted-foreground hover:text-primary inline-flex items-center gap-2 font-mono text-xs tracking-wider transition-colors"
          >
            <CaretLeft weight="bold" size={14} />
            BACK TO BLOCKS
          </Link>
        </div>

        <div className="mb-8 flex items-center justify-between border-2 border-dashed border-primary bg-card/50 p-6">
          <div>
            <h1 className="text-primary mb-2 font-mono text-xl tracking-[0.2em]">
              BLOCK #{block.height}
            </h1>
            <p className="text-muted-foreground font-mono text-xs tracking-wide">
              Finalized block details and metadata
            </p>
          </div>
          <div className="flex items-center gap-4">
            {height > 0 && (
              <Link
                href={`/block/${height - 1}`}
                className="text-muted-foreground hover:text-primary border border-border px-3 py-2 transition-colors hover:border-primary"
                title="Previous block"
              >
                <CaretLeft weight="bold" size={16} />
              </Link>
            )}
            <Link
              href={`/block/${height + 1}`}
              className="text-muted-foreground hover:text-primary border border-border px-3 py-2 transition-colors hover:border-primary"
              title="Next block"
            >
              <CaretRight weight="bold" size={16} />
            </Link>
          </div>
        </div>

        <div className="mb-8 grid gap-6 md:grid-cols-2">
          <TerminalBox title="BLOCK HEADER">
            <DataRow label="HEIGHT" value={block.height} />
            <DataRow label="HASH" value={formatHash(block.hash, 12)} />
            <DataRow
              label="PREV HASH"
              value={formatHash(block.prev_hash, 12)}
            />
            <DataRow
              label="TIMESTAMP"
              value={formatTimestamp(block.timestamp)}
            />
            <DataRow label="TX COUNT" value={block.tx_count} />
          </TerminalBox>

          <TerminalBox title="BLOCK METADATA">
            <DataRow label="PROPOSER" value={formatHash(block.proposer, 12)} />
            <DataRow label="TX ROOT" value={formatHash(block.tx_root, 12)} />
            <DataRow
              label="STATE ROOT"
              value={formatHash(block.state_root, 12)}
            />
            <DataRow
              label="STATUS"
              value={
                <Badge
                  variant="outline"
                  className="border-primary text-primary"
                >
                  FINALIZED
                </Badge>
              }
              mono={false}
            />
          </TerminalBox>
        </div>

        <TerminalBox
          title="TRANSACTIONS"
          subtitle={`${block.tx_count} transactions in this block`}
        >
          {block.tx_count === 0 ? (
            <div className="text-muted-foreground py-8 text-center font-mono text-xs tracking-wider">
              [NO TRANSACTIONS IN THIS BLOCK]
            </div>
          ) : (
            <div className="text-muted-foreground py-8 text-center font-mono text-xs tracking-wider">
              [TX DETAILS COMING SOON]
            </div>
          )}
        </TerminalBox>

        <div className="mt-6 border-t border-border/50 pt-6">
          <div className="text-muted-foreground space-y-2 font-mono text-[10px] tracking-wider">
            <div>FULL HASH: {block.hash}</div>
            <div>PREV FULL HASH: {block.prev_hash}</div>
          </div>
        </div>
      </main>
    </div>
  );
}
