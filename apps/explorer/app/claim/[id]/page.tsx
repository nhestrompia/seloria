"use client";

import { ExplorerHeader } from "@/components/explorer-header";
import {
  DataRow,
  StatBox,
  TerminalBox,
} from "@/components/terminal-components";
import { Badge } from "@/components/ui/badge";
import { formatHash, formatTimestamp } from "@/lib/format";
import { useClaim } from "@/lib/queries";
import { CaretLeft } from "@phosphor-icons/react";
import Link from "next/link";
import { useParams } from "next/navigation";

export default function ClaimDetailPage() {
  const params = useParams();
  const claimId = params.id as string;

  const { data: claim, isLoading: loading, error } = useClaim(claimId);

  if (loading) {
    return (
      <div className="min-h-screen">
        <ExplorerHeader />
        <main className="mx-auto max-w-7xl px-6 py-12">
          <div className="flex items-center justify-center py-20">
            <div className="text-primary border-2 border-dashed border-primary p-6 font-mono text-xs tracking-[0.3em]">
              [LOADING CLAIM...]
            </div>
          </div>
        </main>
      </div>
    );
  }

  if (error || (!loading && !claim)) {
    return (
      <div className="min-h-screen">
        <ExplorerHeader />
        <main className="mx-auto max-w-7xl px-6 py-12">
          <div className="border-2 border-dashed border-destructive bg-destructive/10 p-8 text-center">
            <div className="text-destructive mb-4 font-mono text-xl tracking-[0.3em]">
              [ERROR]
            </div>
            <p className="text-muted-foreground font-mono text-xs tracking-wider">
              {error?.message || "Claim not found"}
            </p>
            <Link
              href="/claims"
              className="text-primary hover:text-accent mt-6 inline-flex items-center gap-2 font-mono text-xs tracking-wider transition-colors"
            >
              <CaretLeft weight="bold" size={14} />
              BACK TO CLAIMS
            </Link>
          </div>
        </main>
      </div>
    );
  }

  if (!claim) {
    return null;
  }

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "pending":
        return (
          <Badge variant="outline" className="border-accent text-accent">
            PENDING
          </Badge>
        );
      case "finalized_yes":
        return (
          <Badge variant="outline" className="border-primary text-primary">
            FINALIZED YES
          </Badge>
        );
      case "finalized_no":
        return (
          <Badge
            variant="outline"
            className="border-destructive text-destructive"
          >
            FINALIZED NO
          </Badge>
        );
      default:
        return <Badge variant="outline">{status.toUpperCase()}</Badge>;
    }
  };

  const totalStake = claim.yes_stake + claim.no_stake;
  const yesPercent = totalStake > 0 ? (claim.yes_stake / totalStake) * 100 : 0;
  const noPercent = totalStake > 0 ? (claim.no_stake / totalStake) * 100 : 0;

  return (
    <div className="min-h-screen">
      <ExplorerHeader />

      <main className="mx-auto max-w-7xl px-6 py-12">
        <div className="mb-6">
          <Link
            href="/claims"
            className="text-muted-foreground hover:text-primary inline-flex items-center gap-2 font-mono text-xs tracking-wider transition-colors"
          >
            <CaretLeft weight="bold" size={14} />
            BACK TO CLAIMS
          </Link>
        </div>

        <div className="mb-8 flex items-center justify-between border-2 border-dashed border-primary bg-card/50 p-6">
          <div>
            <h1 className="text-primary mb-2 font-mono text-xl tracking-[0.2em]">
              CLAIM
            </h1>
            <p className="text-muted-foreground font-mono text-xs tracking-wide">
              Stake-weighted attestation details
            </p>
          </div>
          {getStatusBadge(claim.status)}
        </div>

        <div className="mb-8 grid gap-6 md:grid-cols-4">
          <StatBox
            label="YES STAKE"
            value={claim.yes_stake.toLocaleString()}
            sublabel={`${yesPercent.toFixed(1)}% of total`}
            className="border-primary"
          />
          <StatBox
            label="NO STAKE"
            value={claim.no_stake.toLocaleString()}
            sublabel={`${noPercent.toFixed(1)}% of total`}
            className="border-destructive"
          />
          <StatBox
            label="CREATOR STAKE"
            value={claim.creator_stake.toLocaleString()}
            sublabel="Initial stake"
            className="border-accent"
          />
          <StatBox
            label="ATTESTATIONS"
            value={claim.attestation_count}
            sublabel="Total votes"
          />
        </div>

        <div className="mb-8">
          <TerminalBox title="STAKE VISUALIZATION">
            <div className="space-y-4">
              <div>
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-primary font-mono text-xs tracking-wider">
                    YES STAKE
                  </span>
                  <span className="text-muted-foreground font-mono text-xs">
                    {claim.yes_stake.toLocaleString()}
                  </span>
                </div>
                <div className="h-6 border border-border bg-muted">
                  <div
                    className="h-full bg-primary/30 border-r-2 border-primary"
                    style={{ width: `${yesPercent}%` }}
                  />
                </div>
              </div>
              <div>
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-destructive font-mono text-xs tracking-wider">
                    NO STAKE
                  </span>
                  <span className="text-muted-foreground font-mono text-xs">
                    {claim.no_stake.toLocaleString()}
                  </span>
                </div>
                <div className="h-6 border border-border bg-muted">
                  <div
                    className="h-full bg-destructive/30 border-r-2 border-destructive"
                    style={{ width: `${noPercent}%` }}
                  />
                </div>
              </div>
            </div>

            <div className="text-muted-foreground mt-6 border-t border-border/50 pt-4 font-mono text-[10px] tracking-wider">
              FINALITY THRESHOLD: {(claim.creator_stake * 2).toLocaleString()}{" "}
              tokens
            </div>
          </TerminalBox>
        </div>

        <div className="mb-8 grid gap-6 md:grid-cols-2">
          <TerminalBox title="CLAIM DETAILS">
            <DataRow label="CLAIM ID" value={formatHash(claim.id, 12)} />
            <DataRow label="TYPE" value={claim.claim_type} />
            <DataRow
              label="PAYLOAD HASH"
              value={formatHash(claim.payload_hash, 12)}
            />
            <DataRow
              label="CREATED AT"
              value={formatTimestamp(claim.created_at)}
            />
          </TerminalBox>

          <TerminalBox title="CREATOR">
            <DataRow label="AGENT" value={formatHash(claim.creator, 12)} />
            <DataRow
              label="INITIAL STAKE"
              value={claim.creator_stake.toLocaleString()}
            />
            <DataRow
              label="STATUS"
              value={getStatusBadge(claim.status)}
              mono={false}
            />
          </TerminalBox>
        </div>

        <TerminalBox
          title="ATTESTATIONS"
          subtitle={`${claim.attestation_count} total attestations`}
        >
          <div className="text-muted-foreground py-12 text-center font-mono text-xs tracking-wider">
            [ATTESTATION DETAILS COMING SOON]
          </div>
        </TerminalBox>

        <div className="mt-6 border-t border-border/50 pt-6">
          <div className="text-muted-foreground space-y-2 font-mono text-[10px] tracking-wider">
            <div>FULL CLAIM ID: {claim.id}</div>
            <div>FULL PAYLOAD HASH: {claim.payload_hash}</div>
            <div>FULL CREATOR: {claim.creator}</div>
          </div>
        </div>
      </main>
    </div>
  );
}
