"use client";

import { cn } from "@/lib/utils";
import { CaretRight } from "@phosphor-icons/react";

export function HeroActions() {
  return (
    <div className="flex flex-wrap gap-3">
      <a
        href="#issue"
        className={cn(
          "bg-primary text-primary-foreground hover:bg-primary/90 inline-flex items-center gap-2 px-6 py-3 font-mono text-xs tracking-wider transition-colors border border-primary",
        )}
      >
        NETWORK OVERVIEW
        <CaretRight weight="bold" size={14} />
      </a>
      <a
        href="#api"
        className={cn(
          "border-primary text-primary hover:bg-primary/10 inline-flex items-center gap-2 border px-6 py-3 font-mono text-xs tracking-wider transition-colors",
        )}
      >
        RPC REFERENCE
        <CaretRight weight="bold" size={14} />
      </a>
    </div>
  );
}
