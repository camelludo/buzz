import {
  ArrowUpRight,
  Check,
  CircleAlert,
  Clock3,
  FileText,
  PencilLine,
} from "lucide-react";
import * as React from "react";

import {
  type ReviewDecision,
  parseReviewCard,
} from "@/features/evidence-cards/lib/triageReviewCards";
import type { TimelineMessage } from "@/features/messages/types";
import { Badge } from "@/shared/ui/badge";
import { cn } from "@/shared/lib/cn";

const decisionPresentation = {
  pending: {
    label: "Pending",
    icon: Clock3,
    badgeVariant: "info" as const,
    shell: "border-blue-500/30 bg-blue-500/8 text-blue-800 dark:text-blue-200",
    iconShell: "bg-blue-500/15 text-blue-700 dark:text-blue-300",
  },
  approved: {
    label: "Approved",
    icon: Check,
    badgeVariant: "success" as const,
    shell:
      "border-emerald-500/30 bg-emerald-500/8 text-emerald-800 dark:text-emerald-200",
    iconShell: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
  },
  changes_requested: {
    label: "Changes requested",
    icon: PencilLine,
    badgeVariant: "warning" as const,
    shell:
      "border-amber-500/30 bg-amber-500/8 text-amber-900 dark:text-amber-200",
    iconShell: "bg-amber-500/15 text-amber-700 dark:text-amber-300",
  },
  rejected: {
    label: "Rejected",
    icon: CircleAlert,
    badgeVariant: "destructive" as const,
    shell: "border-destructive/30 bg-destructive/8 text-destructive",
    iconShell: "bg-destructive/15 text-destructive",
  },
} satisfies Record<
  ReviewDecision,
  {
    label: string;
    icon: typeof Check;
    badgeVariant: "info" | "success" | "destructive" | "warning";
    shell: string;
    iconShell: string;
  }
>;

function isHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

export function ReviewCard({ message }: { message: TimelineMessage }) {
  const parsed = React.useMemo(
    () => parseReviewCard(message.tags ?? []),
    [message.tags],
  );

  if (!parsed) {
    return <p className="text-sm text-destructive">Invalid review card.</p>;
  }

  const { payload } = parsed;
  const presentation = decisionPresentation[payload.decision];
  const Icon = presentation.icon;
  const patchIsLink = isHttpUrl(payload.patch_ref);

  return (
    <section
      className="mt-2 overflow-hidden rounded-2xl border border-border/70 bg-card shadow-sm"
      data-testid="review-card"
    >
      <div className="border-b border-border/60 bg-gradient-to-br from-primary/10 via-card to-card p-4">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Badge variant="secondary">Review</Badge>
          <Badge variant={presentation.badgeVariant}>
            {presentation.label}
          </Badge>
          {payload.shadow ? <Badge variant="info">Shadow</Badge> : null}
          <span className="ml-auto font-mono text-2xs text-muted-foreground">
            {payload.review_id.slice(0, 8)}
          </span>
        </div>
        <h3 className="text-base font-semibold tracking-tight">
          {payload.title}
        </h3>
        {payload.reviewer ? (
          <p className="mt-1 text-sm text-muted-foreground">
            Reviewer: {payload.reviewer}
          </p>
        ) : null}
      </div>

      <div className="space-y-3 p-4">
        <div
          className={cn(
            "flex items-start gap-3 rounded-xl border p-3",
            presentation.shell,
          )}
        >
          <span className={cn("rounded-full p-2", presentation.iconShell)}>
            <Icon className="size-4" />
          </span>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-semibold">{presentation.label}</p>
            {patchIsLink ? (
              <a
                className="mt-1 inline-flex items-center gap-1 text-sm font-medium hover:underline"
                href={payload.patch_ref}
                rel="noreferrer"
                target="_blank"
              >
                Open patch <ArrowUpRight className="size-3" />
              </a>
            ) : (
              <p className="mt-1 font-mono text-sm break-all">
                {payload.patch_ref}
              </p>
            )}
            {payload.shadow ? (
              <p className="mt-1 text-xs opacity-70">
                Durable Buzz review · SHADOW / NOT DELIVERED
              </p>
            ) : null}
          </div>
        </div>

        {payload.test_summary ? (
          <div className="rounded-xl bg-blue-500/8 p-3">
            <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-blue-700 dark:text-blue-300">
              <FileText className="size-4" /> Tests
            </div>
            <p className="text-sm">{payload.test_summary}</p>
          </div>
        ) : null}

        {payload.evidence.length > 0 ? (
          <div>
            <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Evidence
            </p>
            <ul className="space-y-1.5">
              {payload.evidence.map((item) => (
                <li
                  className="rounded-lg border border-border/60 bg-muted/30 px-3 py-2 text-sm"
                  key={item}
                >
                  {item}
                </li>
              ))}
            </ul>
          </div>
        ) : null}

        {payload.record_url ? (
          <a
            className="inline-flex items-center gap-1 text-xs font-medium text-primary hover:underline"
            href={payload.record_url}
            rel="noreferrer"
            target="_blank"
          >
            Open record <ArrowUpRight className="size-3" />
          </a>
        ) : null}
      </div>
    </section>
  );
}
