"use client";

import { cn } from "@/lib/utils";
import Link from "next/link";
import { usePathname } from "next/navigation";

export function ExplorerHeader() {
  const pathname = usePathname();

  const navItems = [
    { href: "/", label: "OVERVIEW" },
    { href: "/blocks", label: "BLOCKS" },
    { href: "/transactions", label: "TRANSACTIONS" },
    { href: "/accounts", label: "ACCOUNTS" },
    { href: "/claims", label: "CLAIMS" },
  ];

  return (
    <header className="border-b border-border/60 bg-card/30">
      <div className="mx-auto flex w-full max-w-7xl items-center justify-between px-6 py-6">
        <div className="flex items-center gap-4">
          <div className="border-2 border-primary border-dashed p-3">
            <h1 className="border-primary text-primary font-mono text-base tracking-tighter">
              SELORIA
            </h1>
          </div>
          <div className="flex flex-col">
            <span className="text-primary text-xs font-mono tracking-[0.3em]">
              EXPLORER
            </span>
            <span className="text-muted-foreground text-[10px] font-mono tracking-[0.2em]">
              AGENT RUNTIME
            </span>
          </div>
        </div>

        <nav className="hidden items-center gap-1 md:flex">
          {navItems.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className={cn(
                "px-4 py-2 text-[11px] font-mono tracking-[0.2em] transition-colors border border-transparent",
                pathname === item.href
                  ? "text-primary border-primary bg-primary/5"
                  : "text-muted-foreground hover:text-primary hover:border-primary/50",
              )}
            >
              {item.label}
            </Link>
          ))}
        </nav>
      </div>
    </header>
  );
}
