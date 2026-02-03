import { cn } from "@/lib/utils";
import { ReactNode } from "react";

interface TerminalBoxProps {
  children: ReactNode;
  title?: string;
  subtitle?: string;
  className?: string;
}

export function TerminalBox({
  children,
  title,
  subtitle,
  className,
}: TerminalBoxProps) {
  return (
    <div
      className={cn(
        "border-2 border-dashed border-border bg-card/50 p-6",
        className,
      )}
    >
      {(title || subtitle) && (
        <div className="mb-4 flex items-center justify-between border-b border-border/50 pb-3">
          <div>
            {title && (
              <h3 className="text-primary font-mono text-xs tracking-[0.3em]">
                {title}
              </h3>
            )}
            {subtitle && (
              <p className="text-muted-foreground mt-1 font-mono text-[10px] tracking-[0.2em]">
                {subtitle}
              </p>
            )}
          </div>
        </div>
      )}
      {children}
    </div>
  );
}

interface DataRowProps {
  label: string;
  value: ReactNode;
  mono?: boolean;
}

export function DataRow({ label, value, mono = true }: DataRowProps) {
  return (
    <div className="flex items-start justify-between gap-4 border-b border-border/30 py-3 last:border-0">
      <span className="text-muted-foreground text-xs font-mono tracking-wider">
        {label}
      </span>
      <span
        className={cn(
          "text-primary text-xs text-right break-all",
          mono && "font-mono",
        )}
      >
        {value}
      </span>
    </div>
  );
}

interface StatBoxProps {
  label: string;
  value: ReactNode;
  sublabel?: string;
  className?: string;
}

export function StatBox({ label, value, sublabel, className }: StatBoxProps) {
  return (
    <div
      className={cn(
        "border-2 border-dashed border-border bg-card p-6",
        className,
      )}
    >
      <div className="text-muted-foreground mb-2 font-mono text-[10px] tracking-[0.3em]">
        {label}
      </div>
      <div className="text-primary font-mono text-2xl tracking-tight">
        {value}
      </div>
      {sublabel && (
        <div className="text-muted-foreground mt-1 font-mono text-[10px] tracking-wider">
          {sublabel}
        </div>
      )}
    </div>
  );
}
