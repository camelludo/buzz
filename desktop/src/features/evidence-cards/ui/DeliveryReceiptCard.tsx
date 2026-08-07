import {
  ArrowUpRight,
  Check,
  CircleAlert,
  Send,
  TriangleAlert,
} from "lucide-react";
import * as React from "react";

import {
  type DeliveryReceiptStatus,
  parseDeliveryReceipt,
} from "@/features/evidence-cards/lib/evidenceCards";
import type { TimelineMessage } from "@/features/messages/types";
import { Badge } from "@/shared/ui/badge";
import { cn } from "@/shared/lib/cn";

const statusPresentation = {
  sent: {
    label: "Sent",
    icon: Send,
    badgeVariant: "info" as const,
    shell: "border-blue-500/30 bg-blue-500/8 text-blue-800 dark:text-blue-200",
    iconShell: "bg-blue-500/15 text-blue-700 dark:text-blue-300",
  },
  delivered: {
    label: "Delivered",
    icon: Check,
    badgeVariant: "success" as const,
    shell:
      "border-emerald-500/30 bg-emerald-500/8 text-emerald-800 dark:text-emerald-200",
    iconShell: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
  },
  failed: {
    label: "Failed",
    icon: CircleAlert,
    badgeVariant: "destructive" as const,
    shell: "border-destructive/30 bg-destructive/8 text-destructive",
    iconShell: "bg-destructive/15 text-destructive",
  },
  proof_missing: {
    label: "Proof missing",
    icon: TriangleAlert,
    badgeVariant: "warning" as const,
    shell:
      "border-amber-500/30 bg-amber-500/8 text-amber-900 dark:text-amber-200",
    iconShell: "bg-amber-500/15 text-amber-700 dark:text-amber-300",
  },
} satisfies Record<
  DeliveryReceiptStatus,
  {
    label: string;
    icon: typeof Check;
    badgeVariant: "info" | "success" | "destructive" | "warning";
    shell: string;
    iconShell: string;
  }
>;

export function DeliveryReceiptCard({ message }: { message: TimelineMessage }) {
  const parsed = React.useMemo(
    () => parseDeliveryReceipt(message.tags ?? []),
    [message.tags],
  );

  if (!parsed) {
    return (
      <p className="text-sm text-destructive">Invalid delivery receipt.</p>
    );
  }

  const { payload } = parsed;
  const presentation = statusPresentation[payload.status];
  const Icon = presentation.icon;

  return (
    <section
      className="mt-2 overflow-hidden rounded-2xl border border-border/70 bg-card shadow-sm"
      data-testid="delivery-receipt-card"
    >
      <div className="border-b border-border/60 bg-gradient-to-br from-primary/10 via-card to-card p-4">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Badge variant="secondary">Delivery</Badge>
          <Badge variant={presentation.badgeVariant}>
            {presentation.label}
          </Badge>
          {payload.shadow ? <Badge variant="info">Shadow</Badge> : null}
          <span className="ml-auto font-mono text-2xs text-muted-foreground">
            {payload.receipt_id.slice(0, 8)}
          </span>
        </div>
      </div>

      <div className="p-4">
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
            <p className="mt-0.5 text-sm opacity-90">{payload.detail}</p>
            {payload.shadow ? (
              <p className="mt-1 text-xs opacity-70">
                Durable Buzz receipt · SHADOW / NOT DELIVERED
              </p>
            ) : null}
            {payload.proof_url ? (
              <a
                className="mt-2 inline-flex items-center gap-1 text-xs font-medium text-primary hover:underline"
                href={payload.proof_url}
                rel="noreferrer"
                target="_blank"
              >
                Open proof <ArrowUpRight className="size-3" />
              </a>
            ) : null}
          </div>
        </div>
      </div>
    </section>
  );
}
