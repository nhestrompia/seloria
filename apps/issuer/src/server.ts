const PORT = Number(process.env.PORT ?? "3001");
const RPC_URL = process.env.SELORIA_RPC_URL ?? "http://127.0.0.1:8080";
const ISSUER_API_KEY = process.env.ISSUER_API_KEY;

const landingHtml = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width,initial-scale=1" />
    <title>Seloria Issuer</title>
    <style>
      :root {
        color-scheme: light;
        font-family: "Helvetica Neue", Arial, sans-serif;
      }
      body {
        margin: 0;
        padding: 32px;
        background: #f7f4ee;
        color: #1b1b1b;
      }
      .wrap {
        max-width: 880px;
        margin: 0 auto;
      }
      h1 {
        font-size: 32px;
        margin: 0 0 8px;
      }
      p {
        margin: 0 0 16px;
        color: #3a3a3a;
      }
      .panel {
        background: #fff;
        border: 1px solid #e0dcd1;
        border-radius: 12px;
        padding: 20px;
        margin: 16px 0 24px;
        box-shadow: 0 6px 18px rgba(0,0,0,0.08);
      }
      label {
        display: block;
        font-weight: 600;
        margin: 10px 0 6px;
      }
      input, textarea {
        width: 100%;
        padding: 10px 12px;
        border: 1px solid #d5d0c3;
        border-radius: 8px;
        font-size: 14px;
      }
      textarea {
        min-height: 120px;
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
      }
      button {
        margin-top: 14px;
        background: #d9480f;
        color: white;
        border: 0;
        padding: 10px 16px;
        font-size: 14px;
        border-radius: 8px;
        cursor: pointer;
      }
      .row {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 12px;
      }
      .muted {
        font-size: 13px;
        color: #6b6b6b;
      }
      .links {
        display: flex;
        gap: 12px;
        flex-wrap: wrap;
      }
      .links a {
        color: #d9480f;
        text-decoration: none;
        font-weight: 600;
      }
      pre {
        background: #1e1e1e;
        color: #e6e6e6;
        padding: 12px;
        border-radius: 10px;
        overflow: auto;
      }
    </style>
  </head>
  <body>
    <div class="wrap">
      <h1>Seloria Issuer</h1>
      <p>Issue agent certificates for Seloria. This page calls the issuer API hosted here, which proxies to the Seloria node.</p>
      <div class="links">
        <a href="https://example.com" rel="noreferrer noopener">Explorer (coming soon)</a>
        <a href="https://example.com" rel="noreferrer noopener">Docs</a>
      </div>
      <div class="panel">
        <h3>Issue Certificate</h3>
        <div class="row">
          <div>
            <label>Agent Pubkey</label>
            <input id="agent_pubkey" placeholder="hex public key" />
          </div>
          <div>
            <label>Issued At</label>
            <input id="issued_at" placeholder="unix seconds" value="0" />
          </div>
          <div>
            <label>Expires At</label>
            <input id="expires_at" placeholder="unix seconds" value="2000000000" />
          </div>
        </div>
        <label>Capabilities (comma separated)</label>
        <input id="capabilities" value="TxSubmit,Claim,Attest,KvWrite" />
        <label>Metadata Hash (optional)</label>
        <input id="metadata_hash" placeholder="hex hash or blank" />
        <button id="issue_btn">Issue</button>
        <p class="muted">If ISSUER_API_KEY is set, include header <code>X-Issuer-Key</code>.</p>
      </div>
      <div class="panel">
        <h3>Response</h3>
        <pre id="response">{}</pre>
      </div>
    </div>
    <script>
      const issueBtn = document.getElementById("issue_btn");
      issueBtn.addEventListener("click", async () => {
        const payload = {
          agent_pubkey: document.getElementById("agent_pubkey").value.trim(),
          issued_at: Number(document.getElementById("issued_at").value || "0"),
          expires_at: Number(document.getElementById("expires_at").value || "0"),
          capabilities: document.getElementById("capabilities").value.split(",").map(v => v.trim()).filter(Boolean),
          metadata_hash: document.getElementById("metadata_hash").value.trim() || null
        };
        const resEl = document.getElementById("response");
        resEl.textContent = "Loading...";
        const headers = { "Content-Type": "application/json" };
        const apiKey = localStorage.getItem("issuer_api_key");
        if (apiKey) headers["X-Issuer-Key"] = apiKey;
        try {
          const resp = await fetch("/api/issue", {
            method: "POST",
            headers,
            body: JSON.stringify(payload)
          });
          const text = await resp.text();
          resEl.textContent = text;
        } catch (err) {
          resEl.textContent = String(err);
        }
      });
    </script>
  </body>
</html>`;

Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);

    if (url.pathname === "/" && req.method === "GET") {
      return new Response(landingHtml, {
        headers: { "Content-Type": "text/html; charset=utf-8" }
      });
    }

    if (url.pathname === "/health") {
      return Response.json({ ok: true });
    }

    if (url.pathname === "/api/issue" && req.method === "POST") {
      if (ISSUER_API_KEY) {
        const key = req.headers.get("x-issuer-key");
        if (!key || key !== ISSUER_API_KEY) {
          return new Response("Unauthorized", { status: 401 });
        }
      }

      const body = await req.text();
      const upstream = await fetch(`${RPC_URL}/cert/issue`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body
      });
      const contentType = upstream.headers.get("content-type") ?? "application/json";
      const text = await upstream.text();
      return new Response(text, { status: upstream.status, headers: { "Content-Type": contentType } });
    }

    return new Response("Not Found", { status: 404 });
  }
});

console.log(\`Issuer service listening on http://127.0.0.1:\${PORT}\`);
