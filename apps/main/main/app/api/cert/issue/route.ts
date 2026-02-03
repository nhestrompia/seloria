import { NextResponse } from "next/server";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

type IssueCertRequest = {
  agent_pubkey: string;
  issued_at?: number;
  expires_at?: number;
  capabilities: string[];
  metadata_hash?: string | null;
};

export async function POST(request: Request) {
  const rpcUrl = process.env.SELORIA_RPC_URL;
  if (!rpcUrl) {
    return NextResponse.json(
      { error: "SELORIA_RPC_URL is not configured" },
      { status: 500 },
    );
  }

  const issuerKey = process.env.ISSUER_API_KEY;
  if (issuerKey) {
    const providedKey = request.headers.get("x-issuer-key");
    if (!providedKey || providedKey !== issuerKey) {
      return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }
  }

  let payload: IssueCertRequest;
  try {
    payload = (await request.json()) as IssueCertRequest;
  } catch {
    return NextResponse.json({ error: "Invalid JSON payload" }, { status: 400 });
  }

  if (
    !payload?.agent_pubkey ||
    !Array.isArray(payload.capabilities) ||
    payload.capabilities.length === 0
  ) {
    return NextResponse.json(
      { error: "agent_pubkey and capabilities are required" },
      { status: 400 },
    );
  }

  const now = Math.floor(Date.now() / 1000);
  const issuedAt = payload.issued_at ?? now;
  const expiresAt = payload.expires_at ?? now + 90 * 24 * 60 * 60;

  const upstreamBody = {
    agent_pubkey: payload.agent_pubkey.trim(),
    issued_at: issuedAt,
    expires_at: expiresAt,
    capabilities: payload.capabilities,
    metadata_hash: payload.metadata_hash ?? null,
  };

  try {
    const upstream = await fetch(`${rpcUrl.replace(/\/$/, "")}/cert/issue`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(upstreamBody),
    });

    const text = await upstream.text();
    let data: unknown = {};
    try {
      data = text ? JSON.parse(text) : {};
    } catch {
      data = { error: text || "Upstream returned invalid JSON" };
    }

    return NextResponse.json(data, { status: upstream.status });
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Upstream error" },
      { status: 502 },
    );
  }
}
