"use client";

import { type FormEvent, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

type IssueRequest = {
  agent_pubkey: string;
  issued_at: number;
  expires_at: number;
  capabilities: string[];
  metadata_hash?: string;
};

const CAPABILITIES = [
  { id: "TxSubmit", label: "TxSubmit" },
  { id: "Claim", label: "Claim" },
  { id: "Attest", label: "Attest" },
  { id: "KvWrite", label: "KvWrite" },
];

export function CertificateForm() {
  const [agentPubkey, setAgentPubkey] = useState("");
  const [metadataHash, setMetadataHash] = useState("");
  const [validDays, setValidDays] = useState(90);
  const [issuerKey, setIssuerKey] = useState("");
  const [selectedCaps, setSelectedCaps] = useState<string[]>([
    "TxSubmit",
    "Claim",
    "Attest",
  ]);
  const [loading, setLoading] = useState(false);
  const [response, setResponse] = useState("");
  const [error, setError] = useState("");

  const preview = useMemo(() => {
    const now = Math.floor(Date.now() / 1000);
    const expiresAt = now + Math.max(1, validDays) * 24 * 60 * 60;
    const payload: IssueRequest = {
      agent_pubkey: agentPubkey.trim(),
      issued_at: now,
      expires_at: expiresAt,
      capabilities: selectedCaps,
    };
    if (metadataHash.trim()) {
      payload.metadata_hash = metadataHash.trim();
    }
    return payload;
  }, [agentPubkey, metadataHash, validDays, selectedCaps]);

  const canSubmit =
    agentPubkey.trim().length > 0 && selectedCaps.length > 0 && validDays > 0;

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoading(true);
    setResponse("");
    setError("");

    try {
      const res = await fetch("/api/cert/issue", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...(issuerKey.trim() ? { "X-Issuer-Key": issuerKey.trim() } : {}),
        },
        body: JSON.stringify(preview),
      });

      const text = await res.text();
      let data: unknown = {};
      try {
        data = text ? JSON.parse(text) : {};
      } catch {
        data = { error: text || "Invalid JSON response" };
      }

      if (!res.ok) {
        const message =
          typeof (data as { error?: string })?.error === "string"
            ? (data as { error: string }).error
            : "Certificate issuance failed";
        setError(message);
        return;
      }
      setResponse(JSON.stringify(data, null, 2));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Request failed");
    } finally {
      setLoading(false);
    }
  }

  async function copyResponse() {
    if (!response) return;
    await navigator.clipboard.writeText(response);
  }

  return (
    <div className="border-2 border-dashed border-border bg-card p-6">
      <div className="mb-6 border-b border-border/50 pb-4">
        <h2 className="text-primary mb-2 font-mono text-lg tracking-[0.2em]">
          ISSUE AGENT CERTIFICATE
        </h2>
        <p className="text-muted-foreground font-mono text-xs leading-relaxed">
          The node signs a certificate with the configured issuer key. Use the
          payload in an on-chain registration transaction.
        </p>
      </div>
      <div className="space-y-6">
        <form onSubmit={handleSubmit} className="space-y-5">
          <div className="space-y-2">
            <Label
              htmlFor="agent-pubkey"
              className="font-mono text-xs tracking-wider text-muted-foreground"
            >
              AGENT PUBLIC KEY (HEX)
            </Label>
            <Input
              id="agent-pubkey"
              placeholder="e.g. 0b73...c9"
              value={agentPubkey}
              onChange={(event) => setAgentPubkey(event.target.value)}
              required
              className="border-border bg-input text-foreground font-mono text-xs"
            />
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label
                htmlFor="valid-days"
                className="font-mono text-xs tracking-wider text-muted-foreground"
              >
                VALIDITY (DAYS)
              </Label>
              <Input
                id="valid-days"
                type="number"
                min={1}
                max={3650}
                value={validDays}
                onChange={(event) => setValidDays(Number(event.target.value))}
                className="border-border bg-input text-foreground font-mono text-xs"
              />
            </div>
            <div className="space-y-2">
              <Label
                htmlFor="metadata-hash"
                className="font-mono text-xs tracking-wider text-muted-foreground"
              >
                METADATA HASH (OPTIONAL)
              </Label>
              <Input
                id="metadata-hash"
                placeholder="blake3 hash hex"
                value={metadataHash}
                onChange={(event) => setMetadataHash(event.target.value)}
                className="border-border bg-input text-foreground font-mono text-xs"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label className="font-mono text-xs tracking-wider text-muted-foreground">
              CAPABILITIES
            </Label>
            <div className="grid gap-2 md:grid-cols-2">
              {CAPABILITIES.map((cap) => (
                <label
                  key={cap.id}
                  className="border-border hover:bg-primary/5 flex items-center gap-2 border px-3 py-2 font-mono text-xs tracking-wide transition cursor-pointer"
                >
                  <input
                    type="checkbox"
                    className="border-border text-primary h-4 w-4"
                    checked={selectedCaps.includes(cap.id)}
                    onChange={(event) => {
                      if (event.target.checked) {
                        setSelectedCaps((prev) => [...prev, cap.id]);
                      } else {
                        setSelectedCaps((prev) =>
                          prev.filter((item) => item !== cap.id),
                        );
                      }
                    }}
                  />
                  <span className="text-muted-foreground">{cap.label}</span>
                </label>
              ))}
            </div>
          </div>

          <div className="space-y-2">
            <Label
              htmlFor="issuer-key"
              className="font-mono text-xs tracking-wider text-muted-foreground"
            >
              ISSUER ACCESS KEY (OPTIONAL)
            </Label>
            <Input
              id="issuer-key"
              placeholder="X-Issuer-Key header"
              value={issuerKey}
              onChange={(event) => setIssuerKey(event.target.value)}
              className="border-border bg-input text-foreground font-mono text-xs"
            />
          </div>

          <Button
            type="submit"
            size="lg"
            disabled={!canSubmit || loading}
            className="bg-primary text-primary-foreground hover:bg-primary/90 w-full border border-primary font-mono text-xs tracking-wider"
          >
            {loading ? "[ISSUING...]" : "ISSUE CERTIFICATE"}
          </Button>
        </form>

        <div className="border border-border bg-muted/30 p-4">
          <div className="mb-3 border-b border-border/50 pb-2">
            <h3 className="text-primary font-mono text-[10px] tracking-[0.3em]">
              REQUEST PREVIEW
            </h3>
          </div>
          <pre className="bg-background text-primary max-h-56 overflow-auto border border-border p-3 font-mono text-[11px] leading-relaxed">
            {JSON.stringify(preview, null, 2)}
          </pre>
        </div>

        {(response || error) && (
          <div className="border border-border bg-muted/30 p-4">
            <div className="mb-3 flex items-center justify-between border-b border-border/50 pb-2">
              <h3 className="text-primary font-mono text-[10px] tracking-[0.3em]">
                RESPONSE
              </h3>
              <Button
                variant="ghost"
                size="sm"
                type="button"
                onClick={copyResponse}
                className="text-accent hover:text-primary font-mono text-[10px] tracking-wider"
              >
                COPY
              </Button>
            </div>
            <pre
              className={`max-h-72 overflow-auto border border-border p-3 font-mono text-[11px] leading-relaxed ${error ? "bg-destructive/10 text-destructive" : "bg-background text-primary"}`}
            >
              {error || response}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
