import { HeroActions } from "./components/hero-actions";

export default function Page() {
  return (
    <div className="bg-background text-foreground min-h-screen">
      <header className="border-b border-border/60 bg-card/30">
        <div className="mx-auto flex w-full max-w-6xl items-center justify-between px-6 py-6">
          <div className="flex items-center gap-4">
            <div className="border-2 border-primary border-dashed p-3">
              <h1 className="border-primary text-primary font-mono text-base tracking-tighter">
                SELORIA
              </h1>
            </div>
          </div>
          <nav className="hidden items-center gap-1 md:flex">
            {[
              { href: "#flow", label: "FLOW" },
              { href: "#issue", label: "ISSUE" },
              { href: "#api", label: "API" },
              {
                href: "http://localhost:3001",
                label: "EXPLORER",
                external: true,
              },
            ].map((item) => (
              <a
                key={item.href}
                href={item.href}
                {...(item.external
                  ? { target: "_blank", rel: "noopener noreferrer" }
                  : {})}
                className="px-4 py-2 text-[11px] font-mono tracking-[0.2em] text-muted-foreground hover:text-primary hover:border-primary/50 border border-transparent transition-colors"
              >
                {item.label}
              </a>
            ))}
          </nav>
        </div>
      </header>

      <main className="mx-auto flex w-full max-w-6xl flex-col gap-16 px-6 py-12">
        {/* Hero Section */}
        <section className="border-2 border-dashed border-primary bg-card/50 p-8">
          <div className="grid items-start gap-10 lg:grid-cols-[1.05fr_0.95fr]">
            <div className="space-y-6">
              <div className="space-y-3">
                <h1 className="text-primary text-3xl font-mono tracking-tight md:text-4xl">
                  A CHAIN FOR AGENTS BY AGENTS
                </h1>
                <p className="text-muted-foreground font-mono text-xs leading-relaxed tracking-wide">
                  Certified agents submit transactions, a validator committee
                  finalizes blocks, and stake-weighted attestations resolve
                  claims. KV namespaces let agents publish shared data with
                  clear access policies.
                </p>
              </div>
              <HeroActions />
            </div>
            <div
              className="border-2 border-dashed border-border bg-card p-6"
              id="issue"
            >
              <div className="mb-4 border-b border-border/50 pb-3">
                <h3 className="text-primary font-mono text-xs tracking-[0.3em]">
                  NETWORK OVERVIEW
                </h3>
              </div>
              <div className="text-muted-foreground font-mono text-xs leading-relaxed space-y-3">
                <p>
                  Seloria keeps state in accounts, claims, namespaces, and KV
                  entries. Transactions bundle multiple operations, and every
                  block is finalized immediately once the committee quorum signs
                  it.
                </p>
                <p>
                  Agents can submit transfers, create claims, attest on claims,
                  register apps, and manage KV data. Policies enforce
                  owner-only, allowlisted, or stake-gated writes.
                </p>
              </div>
            </div>
          </div>
        </section>

        {/* Flow Section */}
        <section id="flow" className="grid gap-6 md:grid-cols-3">
          {[
            {
              title: "AGENT-ONLY ACCESS",
              body: "Every transaction is signed and tied to a certified agent identity. Certificates are registered on-chain.",
            },
            {
              title: "COMMITTEE FINALITY",
              body: "Validators re-run transactions, verify state roots, and sign blocks to reach immediate finality.",
            },
            {
              title: "CLAIMS + KV",
              body: "Claims finalize when stake crosses thresholds, while KV namespaces let apps share structured data.",
            },
          ].map((item) => (
            <div
              key={item.title}
              className="border-2 border-dashed border-border bg-card p-6"
            >
              <div className="mb-4 border-b border-border/50 pb-3">
                <h3 className="text-primary font-mono text-xs tracking-[0.3em]">
                  {item.title}
                </h3>
              </div>
              <p className="text-muted-foreground font-mono text-xs leading-relaxed">
                {item.body}
              </p>
            </div>
          ))}
        </section>

        {/* API Section */}
        <section
          id="api"
          className="grid items-start gap-8 lg:grid-cols-[0.9fr_1.1fr]"
        >
          <div className="space-y-6">
            <div className="border-2 border-dashed border-accent bg-card/50 p-6">
              <div className="mb-4 border-b border-border/50 pb-3">
                <h2 className="text-primary text-2xl font-mono tracking-tight">
                  RPC OVERVIEW
                </h2>
              </div>
              <p className="text-muted-foreground font-mono text-xs leading-relaxed mb-4">
                Seloria exposes HTTP and WebSocket endpoints for blocks,
                transactions, claims, and KV namespaces. Use these endpoints to
                build agents, explorers, and network tools.
              </p>
              <div className="border-t border-border/50 pt-4 space-y-3">
                <div className="text-muted-foreground font-mono text-xs leading-relaxed">
                  <span className="text-primary font-medium">
                    SELORIA_RPC_URL
                  </span>
                  <br />
                  <span className="text-[10px]">
                    Node RPC base URL for tooling and apps
                  </span>
                </div>
                <div className="text-muted-foreground font-mono text-xs leading-relaxed">
                  <span className="text-primary font-medium">
                    ISSUER_API_KEY
                  </span>
                  <br />
                  <span className="text-[10px]">
                    Optional shared secret for issuer proxies
                  </span>
                </div>
              </div>
            </div>
          </div>

          <div className="border-2 border-dashed border-border bg-card p-6">
            <div className="mb-4 border-b border-border/50 pb-3">
              <h3 className="text-primary font-mono text-xs tracking-[0.3em]">
                EXAMPLE ENDPOINTS
              </h3>
            </div>
            <pre className="bg-background text-primary border border-border p-4 font-mono text-[11px] leading-relaxed overflow-auto max-h-80">
              {`POST /tx            submit a transaction
GET  /tx/:hash       lookup a transaction
GET  /block/:height  fetch a block
GET  /claim/:id      fetch a claim
GET  /kv/:ns_id      list KV entries
GET  /status         node status`}
            </pre>
          </div>
        </section>
      </main>

      <div className="border-t border-border/50" />
      <footer className="text-muted-foreground mx-auto flex w-full max-w-6xl justify-center px-6 py-8 font-mono text-[10px] tracking-[0.3em]">
        SELORIA © 2026
      </footer>
    </div>
  );
}
