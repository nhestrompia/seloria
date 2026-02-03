#!/usr/bin/env node
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../..");

const args = process.argv.slice(2);
const arg = (key, fallback) => {
  const idx = args.indexOf(`--${key}`);
  if (idx === -1) return fallback;
  return args[idx + 1] ?? fallback;
};

const configPath = resolve(
  repoRoot,
  arg("config", "scripts/llm-activity/config.json"),
);
const steps = Number(arg("steps", "5"));
const intervalMs = Number(arg("interval", "2000"));

const OPENROUTER_API_KEY = process.env.OPENROUTER_API_KEY;
if (!OPENROUTER_API_KEY) {
  console.error("OPENROUTER_API_KEY is required.");
  process.exit(1);
}

const config = JSON.parse(await readFile(configPath, "utf-8"));
const rpcUrl =
  config.rpcUrl ?? process.env.SELORIA_RPC_URL ?? "http://127.0.0.1:8080";
const model = config.model ?? process.env.OPENROUTER_MODEL ?? "openrouter/auto";
const issuerSecret = config.issuerSecret ?? process.env.ISSUER_SECRET;
const seloriaBin = process.env.SELORIA_BIN;

if (!Array.isArray(config.agents) || config.agents.length === 0) {
  console.error("Config must include agents with pubkey + secret.");
  process.exit(1);
}

const outDir = resolve(repoRoot, "scripts/llm-activity/out");
await mkdir(outDir, { recursive: true });

function runCommand(cmd, cmdArgs, opts = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(cmd, cmdArgs, {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
      ...opts,
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (data) => (stdout += data.toString()));
    child.stderr.on("data", (data) => (stderr += data.toString()));
    child.on("close", (code) => {
      if (code === 0) {
        resolve({ stdout, stderr });
      } else {
        reject(new Error(`Command failed (${code}): ${stderr || stdout}`));
      }
    });
  });
}

async function runSeloria(argsList) {
  if (seloriaBin) {
    return runCommand(seloriaBin, argsList);
  }
  return runCommand("cargo", ["run", "--bin", "seloria", "--", ...argsList]);
}

async function fetchJson(path) {
  const res = await fetch(`${rpcUrl.replace(/\/$/, "")}${path}`);
  if (!res.ok) {
    throw new Error(`RPC ${path} failed: ${res.status}`);
  }
  return res.json();
}

async function issueCertIfNeeded(agent) {
  if (!issuerSecret) return;
  const account = await fetchJson(`/account/${agent.pubkey}`);
  if (account.nonce > 0) {
    return;
  }

  const now = Math.floor(Date.now() / 1000);
  const expiresAt = now + 365 * 24 * 60 * 60;
  const outFile = resolve(outDir, `cert-${agent.name ?? agent.pubkey}.json`);

  await runSeloria([
    "txgen",
    "agent-cert",
    "--issuer-secret",
    issuerSecret,
    "--agent-secret",
    agent.secret,
    "--issued-at",
    String(now),
    "--expires-at",
    String(expiresAt),
    "--capabilities",
    "txsubmit,claim,attest,kvwrite",
    "--nonce",
    String(account.nonce + 1),
    "--fee",
    String(config.fee ?? 100),
    "--out",
    outFile,
  ]);

  await runSeloria(["tx", "--endpoint", rpcUrl, "--file", outFile]);
}

async function getNextAction(agent, account, otherAgents) {
  const payload = {
    agent: {
      name: agent.name ?? "agent",
      pubkey: agent.pubkey,
      balance: account.balance,
      nonce: account.nonce,
    },
    other_agents: otherAgents.map((a) => ({
      name: a.name ?? "agent",
      pubkey: a.pubkey,
    })),
    rules: {
      max_transfer: config.maxTransfer ?? 500,
      max_claim_stake: config.maxClaimStake ?? 500,
      max_fee: config.fee ?? 100,
      allow_kv: Boolean(config.namespaceId),
      namespace_id: config.namespaceId ?? null,
    },
  };

  const system = [
    "You are coordinating activity on an agent-only blockchain.",
    "Choose a single action to take next.",
    "Respond with JSON only (no markdown).",
    "Actions: transfer, claim_create, kv_put.",
    "Keep amounts small and within the provided max values.",
  ].join(" ");

  const user = `State:\n${JSON.stringify(payload, null, 2)}\n\nReturn JSON like:
{"action":"transfer","to_pubkey":"...","amount":123}
{"action":"claim_create","claim_type":"signal","payload":"...","stake":123}
{"action":"kv_put","key":"obj/v1/demo/${randomUUID()}","codec":"json","value":"{...}"}`;

  const res = await fetch("https://openrouter.ai/api/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${OPENROUTER_API_KEY}`,
      "Content-Type": "application/json",
      "HTTP-Referer": "https://seloria.local",
      "X-Title": "Seloria LLM Activity",
    },
    body: JSON.stringify({
      model,
      messages: [
        { role: "system", content: system },
        { role: "user", content: user },
      ],
      temperature: 0.7,
    }),
  });

  const data = await res.json();
  const content = data?.choices?.[0]?.message?.content ?? "{}";
  try {
    return JSON.parse(content);
  } catch {
    return { action: "transfer" };
  }
}

function normalizeAction(action, agent, otherAgents) {
  const fee = Number(config.fee ?? 100);
  const maxTransfer = Number(config.maxTransfer ?? 500);
  const maxStake = Number(config.maxClaimStake ?? 500);

  if (action.action === "transfer" && otherAgents.length > 0) {
    const recipient =
      action.to_pubkey ??
      otherAgents[Math.floor(Math.random() * otherAgents.length)].pubkey;
    const amount = Math.max(
      1,
      Math.min(maxTransfer, Number(action.amount ?? 10)),
    );
    return { type: "transfer", to: recipient, amount, fee };
  }

  if (action.action === "claim_create") {
    const stake = Math.max(1, Math.min(maxStake, Number(action.stake ?? 50)));
    return {
      type: "claim_create",
      claim_type: String(action.claim_type ?? "signal"),
      payload: String(action.payload ?? `signal:${randomUUID()}`),
      stake,
      fee,
    };
  }

  if (action.action === "kv_put" && config.namespaceId) {
    return {
      type: "kv_put",
      ns_id: config.namespaceId,
      key: String(action.key ?? `obj/v1/demo/${randomUUID()}`),
      codec: String(action.codec ?? "json"),
      value: String(action.value ?? `{"note":"${randomUUID()}"}`),
      fee,
    };
  }

  if (otherAgents.length > 0) {
    const recipient = otherAgents[0].pubkey;
    return { type: "transfer", to: recipient, amount: 10, fee };
  }

  return {
    type: "claim_create",
    claim_type: "signal",
    payload: `signal:${randomUUID()}`,
    stake: 10,
    fee,
  };
}

async function submitTx(agent, action, nonce) {
  const outFile = resolve(outDir, `tx-${agent.pubkey}-${Date.now()}.json`);

  if (action.type === "transfer") {
    await runSeloria([
      "txgen",
      "transfer",
      "--from-secret",
      agent.secret,
      "--to-pubkey",
      action.to,
      "--amount",
      String(action.amount),
      "--nonce",
      String(nonce),
      "--fee",
      String(action.fee),
      "--out",
      outFile,
    ]);
  } else if (action.type === "claim_create") {
    await runSeloria([
      "txgen",
      "claim-create",
      "--from-secret",
      agent.secret,
      "--claim-type",
      action.claim_type,
      "--payload",
      action.payload,
      "--stake",
      String(action.stake),
      "--nonce",
      String(nonce),
      "--fee",
      String(action.fee),
      "--out",
      outFile,
    ]);
  } else if (action.type === "kv_put") {
    await runSeloria([
      "txgen",
      "kv-put",
      "--from-secret",
      agent.secret,
      "--ns-id",
      action.ns_id,
      "--key",
      action.key,
      "--codec",
      action.codec,
      "--value",
      action.value,
      "--nonce",
      String(nonce),
      "--fee",
      String(action.fee),
      "--out",
      outFile,
    ]);
  } else {
    throw new Error(`Unsupported action: ${action.type}`);
  }

  await runSeloria(["tx", "--endpoint", rpcUrl, "--file", outFile]);
}

for (let i = 0; i < steps; i += 1) {
  const agent = config.agents[i % config.agents.length];
  const otherAgents = config.agents.filter((a) => a.pubkey !== agent.pubkey);

  await issueCertIfNeeded(agent);

  const account = await fetchJson(`/account/${agent.pubkey}`);
  const llmAction = await getNextAction(agent, account, otherAgents);
  const action = normalizeAction(llmAction, agent, otherAgents);

  await submitTx(agent, action, account.nonce + 1);

  if (i < steps - 1) {
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
}

await writeFile(
  resolve(outDir, "last-run.json"),
  JSON.stringify({ steps, completed_at: Date.now() }, null, 2),
);
