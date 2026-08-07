import { ArrowUpRight, Layers3 } from "lucide-react";
import * as React from "react";

import {
  type TriageRisk,
  parseTriageCard,
} from "@/features/evidence-cards/lib/triageReviewCards";
import type { TimelineMessage } from "@/features/messages/types";
import { Badge } from "@/shared/ui/badge";
import { cn } from "@/shared/lib/cn";

const riskPresentation = {
  low: {
    label: "Low",
    badgeVariant: "secondary" as const,
    chip: "border-emerald-500/30 bg-emerald-500/10 text-emerald-800 dark:text-emerald-200",
    count: "bg-emerald-500/15 text-emerald-800 dark:text-emerald-200",
  },
  medium: {
    label: "Medium",
    badgeVariant: "warning" as const,
    chip: "border-amber-500/30 bg-amber-500/10 text-amber-900 dark:text-amber-200",
    count: "bg-amber-500/15 text-amber-900 dark:text-amber-200",
  },
  high: {
    label: "High",
    badgeVariant: "destructive" as const,
    chip: "border-destructive/30 bg-destructive/10 text-destructive",
    count: "bg-destructive/15 text-destructive",
  },
} satisfies Record<
  TriageRisk,
  {
    label: string;
    badgeVariant: "secondary" | "warning" | "destructive";
    chip: string;
    count: string;
  }
>;

export function TriageCard({ message }: { message: TimelineMessage }) {
  const parsed = React.useMemo(
    () => parseTriageCard(message.tags ?? []),
    [message.tags],
  );

  if (!parsed) {
    return <p className="text-sm text-destructive">Invalid triage card.</p>;
  }

  const { payload } = parsed;
  const totalCount = payload.groups.reduce((sum, group) => sum + group.count, 0);

  return (
    <section
      className="mt-2 overflow-hidden rounded-2xl border border-border/70 bg-card shadow-sm"
      data-testid="triage-card"
    >
      <div className="border-b border-border/60 bg-gradient-to-br from-primary/10 via-card to-card p-4">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Badge variant="secondary">Triage</Badge>
          <Badge variant="info">{totalCount} items</Badge>
          {payload.shadow ? <Badge variant="info">Shadow</Badge> : null}
          <span className="ml-auto font-mono text-2xs text-muted-foreground">
            {payload.triage_id.slice(0, 8)}
          </span>
        </div>
        <h3 className="text-base font-semibold tracking-tight">
          {payload.title}
        </h3>
        <p className="mt-1 text-sm text-muted-foreground">
          {payload.proposed_action}
        </p>
        {payload.record_url ? (
          <a
            className="mt-2 inline-flex items-center gap-1 text-xs font-medium text-primary hover:underline"
            href={payload.record_url}
            rel="noreferrer"
            target="_blank"
          >
            Open record <ArrowUpRight className="size-3" />
          </a>
        ) : null}
      </div>

      <div className="space-y-2 p-4">
        <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          <Layers3 className="size-4" /> Groups
        </div>
        {payload.groups.map((group) => {
          const presentation = riskPresentation[group.risk];
          return (
            <div
              className={cn(
                "flex flex-wrap items-center gap-2 rounded-xl border px-3 py-2.5",
                presentation.chip,
              )}
              key={`${group.label}-${group.risk}-${group.count}`}
            >
              <span className="min-w-0 flex-1 text-sm font-medium">
                {group.label}
              </span>
              <span
                className={cn(
                  "rounded-full px-2 py-0.5 font-mono text-xs font-semibold",
                  presentation.count,
                )}
              >
                {group.count}
              </span>
              {group.oldest_age ? (
                <span className="text-xs opacity-80">
                  oldest {group.oldest_age}
                </span>
              ) : null}
              <Badge variant={presentation.badgeVariant}>
                {presentation.label}
              </Badge>
            </div>
          );
        })}
        {payload.shadow ? (
          <p className="pt-1 text-xs text-muted-foreground">
            Durable Buzz triage · SHADOW / NOT DELIVERED
          </p>
        ) : null}
      </div>
    </section>
  );
}
