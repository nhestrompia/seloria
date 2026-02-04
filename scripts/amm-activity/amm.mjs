#!/usr/bin/env node
import { spawn } from "node:child_process";
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
  arg("config", "scripts/amm-activity/config.json"),
);
const rpcUrl = arg("rpc", "");

const config = JSON.parse(await readFile(configPath, "utf-8"));
const endpoint = rpcUrl || config.rpcUrl || "http://127.0.0.1:8080";
const fee = Number(config.fee ?? 100);
const issuerSecret = config.issuerSecret ?? process.env.ISSUER_SECRET;
const seloriaBin = process.env.SELORIA_BIN;
const faucetUrl = config.faucetUrl ?? `${endpoint.replace(/\/$/, "")}/faucet`;
const faucetKey = config.faucetKey ?? process.env.FAUCET_KEY;
const faucetAmount = Number(config.faucetAmount ?? 100000);
const minBalance = Number(config.minBalance ?? 5000);

const agents = config.agents ?? [];
if (!Array.isArray(agents) || agents.length < 2) {
  console.error("Config must include at least 2 agents with pubkey + secret.");
  process.exit(1);
}

const NATIVE_TOKEN_ID = "0".repeat(64);
const outDir = resolve(repoRoot, "scripts/amm-activity/out");
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
  const res = await fetch(`${endpoint.replace(/\/$/, "")}${path}`);
  if (!res.ok) {
    throw new Error(`RPC ${path} failed: ${res.status}`);
  }
  return res.json();
}

function parseId(label, stdout) {
  const re = new RegExp(`${label}=([0-9a-fA-F]+)`);
  const match = stdout.match(re);
  if (!match) {
    return null;
  }
  return match[1].toLowerCase();
}

async function issueCertIfNeeded(agent) {
  if (!issuerSecret) return;
  const account = await fetchJson(`/account/${agent.pubkey}`);
  if (account.nonce > 0) return;

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
    String(fee),
    "--out",
    outFile,
  ]);

  await runSeloria(["tx", "--endpoint", endpoint, "--file", outFile]);
  await waitForNonce(agent.pubkey, account.nonce + 1);
}

async function ensureFunding(agent) {
  const account = await fetchJson(`/account/${agent.pubkey}`);
  if (account.balance >= minBalance) {
    return account;
  }

  if (!faucetUrl) {
    return account;
  }

  const res = await fetch(faucetUrl, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(faucetKey ? { "X-Faucet-Key": faucetKey } : {}),
    },
    body: JSON.stringify({
      to_pubkey: agent.pubkey,
      amount: faucetAmount,
    }),
  });

  if (!res.ok) {
    const text = await res.text();
    console.warn(`Faucet request failed: ${text}`);
    return account;
  }

  return waitForBalance(agent.pubkey, minBalance);
}

async function waitForBalance(pubkey, minExpected) {
  for (let i = 0; i < 20; i += 1) {
    const updated = await fetchJson(`/account/${pubkey}`);
    if (updated.balance >= minExpected) {
      return updated;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  return fetchJson(`/account/${pubkey}`);
}

async function waitForNonce(pubkey, expected) {
  for (let i = 0; i < 20; i += 1) {
    const account = await fetchJson(`/account/${pubkey}`);
    if (account.nonce >= expected) {
      return account;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  return fetchJson(`/account/${pubkey}`);
}

function integerSqrt(n) {
  if (n <= 0) return 0;
  let x0 = BigInt(n);
  let x1 = (x0 + 1n) >> 1n;
  while (x1 < x0) {
    x0 = x1;
    x1 = (x1 + BigInt(n) / x1) >> 1n;
  }
  return Number(x0);
}

async function submitTx(agent, argsList, expectedNonce) {
  await runSeloria(argsList);
  await runSeloria(["tx", "--endpoint", endpoint, "--file", argsList[argsList.length - 1]]);
  await waitForNonce(agent.pubkey, expectedNonce);
}

const [creator, trader] = agents;

await issueCertIfNeeded(creator);
await issueCertIfNeeded(trader);
await ensureFunding(creator);
await ensureFunding(trader);

const tokenConfig = config.token ?? {
  name: "Amm Token",
  symbol: "AMM",
  decimals: 6,
  supply: 1_000_000,
};

const poolConfig = config.pool ?? {
  amountNative: 100_000,
  amountToken: 500_000,
};

const transferConfig = config.transfer ?? { amount: 100_000 };
const swapConfig = config.swap ?? { amountIn: 1_000, minOut: 1 };
const removeConfig = config.remove ?? { lpAmount: 0, minA: 1, minB: 1 };

const creatorAccount = await fetchJson(`/account/${creator.pubkey}`);
const tokenOut = resolve(outDir, `token-create-${Date.now()}.json`);
const tokenCreate = await runSeloria([
  "txgen",
  "token-create",
  "--from-secret",
  creator.secret,
  "--name",
  tokenConfig.name,
  "--symbol",
  tokenConfig.symbol,
  "--decimals",
  String(tokenConfig.decimals),
  "--total-supply",
  String(tokenConfig.supply),
  "--nonce",
  String(creatorAccount.nonce + 1),
  "--fee",
  String(fee),
  "--out",
  tokenOut,
]);
const tokenId = parseId("TOKEN_ID", tokenCreate.stdout);
if (!tokenId) {
  throw new Error("Failed to parse TOKEN_ID from txgen output.");
}
await runSeloria(["tx", "--endpoint", endpoint, "--file", tokenOut]);
await waitForNonce(creator.pubkey, creatorAccount.nonce + 1);

const creatorAfterToken = await fetchJson(`/account/${creator.pubkey}`);
const transferOut = resolve(outDir, `token-transfer-${Date.now()}.json`);
await submitTx(
  creator,
  [
    "txgen",
    "token-transfer",
    "--from-secret",
    creator.secret,
    "--token-id",
    tokenId,
    "--to-pubkey",
    trader.pubkey,
    "--amount",
    String(transferConfig.amount),
    "--nonce",
    String(creatorAfterToken.nonce + 1),
    "--fee",
    String(fee),
    "--out",
    transferOut,
  ],
  creatorAfterToken.nonce + 1,
);

const creatorAfterTransfer = await fetchJson(`/account/${creator.pubkey}`);
const poolOut = resolve(outDir, `pool-create-${Date.now()}.json`);
const poolCreate = await runSeloria([
  "txgen",
  "pool-create",
  "--from-secret",
  creator.secret,
  "--token-a",
  NATIVE_TOKEN_ID,
  "--token-b",
  tokenId,
  "--amount-a",
  String(poolConfig.amountNative),
  "--amount-b",
  String(poolConfig.amountToken),
  "--nonce",
  String(creatorAfterTransfer.nonce + 1),
  "--fee",
  String(fee),
  "--out",
  poolOut,
]);
const poolId = parseId("POOL_ID", poolCreate.stdout);
if (!poolId) {
  throw new Error("Failed to parse POOL_ID from txgen output.");
}
await runSeloria(["tx", "--endpoint", endpoint, "--file", poolOut]);
await waitForNonce(creator.pubkey, creatorAfterTransfer.nonce + 1);

const traderAccount = await fetchJson(`/account/${trader.pubkey}`);
const swapOut = resolve(outDir, `swap-${Date.now()}.json`);
await submitTx(
  trader,
  [
    "txgen",
    "swap",
    "--from-secret",
    trader.secret,
    "--pool-id",
    poolId,
    "--token-in",
    NATIVE_TOKEN_ID,
    "--amount-in",
    String(swapConfig.amountIn),
    "--min-out",
    String(swapConfig.minOut),
    "--nonce",
    String(traderAccount.nonce + 1),
    "--fee",
    String(fee),
    "--out",
    swapOut,
  ],
  traderAccount.nonce + 1,
);

const lpInitial = integerSqrt(
  BigInt(poolConfig.amountNative) * BigInt(poolConfig.amountToken),
);
const removeLp = removeConfig.lpAmount > 0 ? removeConfig.lpAmount : Math.max(1, Math.floor(lpInitial / 10));
const creatorAfterPool = await fetchJson(`/account/${creator.pubkey}`);
const removeOut = resolve(outDir, `pool-remove-${Date.now()}.json`);
await submitTx(
  creator,
  [
    "txgen",
    "pool-remove",
    "--from-secret",
    creator.secret,
    "--pool-id",
    poolId,
    "--lp-amount",
    String(removeLp),
    "--min-a",
    String(removeConfig.minA),
    "--min-b",
    String(removeConfig.minB),
    "--nonce",
    String(creatorAfterPool.nonce + 1),
    "--fee",
    String(fee),
    "--out",
    removeOut,
  ],
  creatorAfterPool.nonce + 1,
);

await writeFile(
  resolve(outDir, "last-run.json"),
  JSON.stringify(
    {
      endpoint,
      token_id: tokenId,
      pool_id: poolId,
      completed_at: Date.now(),
    },
    null,
    2,
  ),
);

console.log(`AMM test completed. token_id=${tokenId} pool_id=${poolId}`);
