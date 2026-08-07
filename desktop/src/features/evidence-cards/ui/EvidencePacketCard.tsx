import {
  ArrowUpRight,
  CircleAlert,
  FileSearch,
  ShieldCheck,
} from "lucide-react";
import * as React from "react";

import { parseEvidencePacket } from "@/features/evidence-cards/lib/evidenceCards";
import type { TimelineMessage } from "@/features/messages/types";
import { Badge } from "@/shared/ui/badge";

export function EvidencePacketCard({ message }: { message: TimelineMessage }) {
  const parsed = React.useMemo(
    () => parseEvidencePacket(message.tags ?? []),
    [message.tags],
  );

  if (!parsed) {
    return <p className="text-sm text-destructive">Invalid evidence packet.</p>;
  }

  const { payload } = parsed;
  const confidencePct = Math.round(payload.confidence * 100);
  const confidenceWidth = `${Math.min(100, Math.max(0, confidencePct))}%`;

  return (
    <section
      className="mt-2 overflow-hidden rounded-2xl border border-border/70 bg-card shadow-sm"
      data-testid="evidence-packet-card"
    >
      <div className="border-b border-border/60 bg-gradient-to-br from-primary/10 via-card to-card p-4">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Badge variant="info">Evidence</Badge>
          {payload.shadow ? <Badge variant="info">Shadow</Badge> : null}
          <span className="ml-auto font-mono text-2xs text-muted-foreground">
            {payload.packet_id.slice(0, 8)}
          </span>
        </div>
        <h3 className="text-base font-semibold tracking-tight">
          {payload.title}
        </h3>
        <p className="mt-1 text-sm text-muted-foreground">
          {payload.parse_summary}
        </p>
        <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1">
          <a
            className="inline-flex items-center gap-1 text-xs font-medium text-primary hover:underline"
            href={payload.source_url}
            rel="noreferrer"
            target="_blank"
          >
            Open source <ArrowUpRight className="size-3" />
          </a>
          {payload.record_url ? (
            <a
              className="inline-flex items-center gap-1 text-xs font-medium text-primary hover:underline"
              href={payload.record_url}
              rel="noreferrer"
              target="_blank"
            >
              Open authoritative record <ArrowUpRight className="size-3" />
            </a>
          ) : null}
        </div>
      </div>

      <div className="grid gap-3 p-4 sm:grid-cols-2">
        <div className="rounded-xl bg-emerald-500/8 p-3">
          <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-emerald-700 dark:text-emerald-300">
            <ShieldCheck className="size-4" /> Recommendation
          </div>
          <p className="text-sm">{payload.recommendation}</p>
        </div>
        <div className="rounded-xl bg-blue-500/8 p-3">
          <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-blue-700 dark:text-blue-300">
            <FileSearch className="size-4" /> Confidence
          </div>
          <div className="mt-2 flex items-center gap-2">
            <div
              aria-hidden="true"
              className="h-2 flex-1 overflow-hidden rounded-full bg-blue-500/15"
            >
              <div
                className="h-full rounded-full bg-blue-500/70 dark:bg-blue-400/70"
                style={{ width: confidenceWidth }}
              />
            </div>
            <span className="shrink-0 font-mono text-xs font-medium text-blue-700 dark:text-blue-300">
              {confidencePct}%
            </span>
          </div>
          <p className="sr-only">Confidence {confidencePct} percent</p>
        </div>
      </div>

      {(payload.red_flags.length > 0 || payload.missing.length > 0) && (
        <div className="space-y-3 px-4 pb-4">
          {payload.red_flags.length > 0 ? (
            <div>
              <div className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-amber-700 dark:text-amber-300">
                <CircleAlert className="size-4" /> Red flags
              </div>
              <div className="flex flex-wrap gap-1.5">
                {payload.red_flags.map((flag) => (
                  <span
                    className="rounded-full border border-amber-500/30 bg-amber-500/10 px-2.5 py-1 text-xs text-amber-800 dark:text-amber-200"
                    key={`flag-${flag}`}
                  >
                    {flag}
                  </span>
                ))}
              </div>
            </div>
          ) : null}
          {payload.missing.length > 0 ? (
            <div>
              <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Missing
              </p>
              <div className="flex flex-wrap gap-1.5">
                {payload.missing.map((item) => (
                  <span
                    className="rounded-full border border-border/60 bg-muted/40 px-2.5 py-1 text-xs text-muted-foreground"
                    key={`missing-${item}`}
                  >
                    {item}
                  </span>
                ))}
              </div>
            </div>
          ) : null}
        </div>
      )}
    </section>
  );
}
