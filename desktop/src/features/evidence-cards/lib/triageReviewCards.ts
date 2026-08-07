import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex } from "@noble/hashes/utils.js";
import { z } from "zod";

const httpUrlSchema = z
  .string()
  .max(2_048)
  .refine((value) => {
    try {
      return ["http:", "https:"].includes(new URL(value).protocol);
    } catch {
      return false;
    }
  });

const boundedText = (max: number) => z.string().trim().min(1).max(max);

export const TRIAGE_RISKS = ["low", "medium", "high"] as const;
export type TriageRisk = (typeof TRIAGE_RISKS)[number];

export const REVIEW_DECISIONS = [
  "pending",
  "approved",
  "changes_requested",
  "rejected",
] as const;
export type ReviewDecision = (typeof REVIEW_DECISIONS)[number];

const triageGroupSchema = z.object({
  label: boundedText(160),
  count: z.number().int().min(1),
  oldest_age: boundedText(64).optional(),
  risk: z.enum(TRIAGE_RISKS),
});

const triageCardPayloadSchema = z.object({
  schema_version: z.literal(1),
  triage_id: z.string().uuid(),
  title: boundedText(160),
  groups: z.array(triageGroupSchema).min(1).max(32),
  proposed_action: boundedText(2_000),
  record_url: httpUrlSchema.optional(),
  shadow: z.boolean(),
});

const reviewCardPayloadSchema = z.object({
  schema_version: z.literal(1),
  review_id: z.string().uuid(),
  title: boundedText(160),
  patch_ref: boundedText(2_048),
  evidence: z.array(boundedText(500)).max(16),
  test_summary: boundedText(2_000).optional(),
  decision: z.enum(REVIEW_DECISIONS),
  reviewer: boundedText(160).optional(),
  record_url: httpUrlSchema.optional(),
  shadow: z.boolean(),
});

export type TriageCardPayload = z.infer<typeof triageCardPayloadSchema>;
export type ReviewCardPayload = z.infer<typeof reviewCardPayloadSchema>;

export type ParsedTriageCard = {
  payload: TriageCardPayload;
  payloadHash: string;
};

export type ParsedReviewCard = {
  payload: ReviewCardPayload;
  payloadHash: string;
};

function findTag(tags: string[][], name: string): string | undefined {
  return tags.find((tag) => tag[0] === name)?.[1];
}

function verifyPayloadHash(
  encoded: string,
  payloadHash: string | undefined,
): string | null {
  if (!payloadHash || !/^[0-9a-f]{64}$/i.test(payloadHash)) {
    return null;
  }
  const encodedHash = bytesToHex(sha256(new TextEncoder().encode(encoded)));
  if (encodedHash !== payloadHash.toLowerCase()) {
    return null;
  }
  return encodedHash;
}

export function parseTriageCard(tags: string[][]): ParsedTriageCard | null {
  const encoded = findTag(tags, "triage_card");
  const payloadHash = findTag(tags, "payload_hash");
  if (!encoded) return null;

  const verifiedHash = verifyPayloadHash(encoded, payloadHash);
  if (!verifiedHash) return null;

  try {
    const payload = triageCardPayloadSchema.parse(JSON.parse(encoded));
    return { payload, payloadHash: verifiedHash };
  } catch {
    return null;
  }
}

export function parseReviewCard(tags: string[][]): ParsedReviewCard | null {
  const encoded = findTag(tags, "review_card");
  const payloadHash = findTag(tags, "payload_hash");
  if (!encoded) return null;

  const verifiedHash = verifyPayloadHash(encoded, payloadHash);
  if (!verifiedHash) return null;

  try {
    const payload = reviewCardPayloadSchema.parse(JSON.parse(encoded));
    return { payload, payloadHash: verifiedHash };
  } catch {
    return null;
  }
}

export function buildTriageCardContent(payload: TriageCardPayload): string {
  triageCardPayloadSchema.parse(payload);
  const groupLines = payload.groups.map((group) => {
    const age = group.oldest_age ? ` · oldest ${group.oldest_age}` : "";
    return `- ${group.label}: ${group.count}${age} (${group.risk})`;
  });
  const lines = [
    `## ${payload.title}`,
    groupLines.join("\n"),
    `**Proposed action:** ${payload.proposed_action}`,
  ];
  if (payload.shadow) {
    lines.push("**SHADOW / NOT DELIVERED**");
  }
  return lines.join("\n\n");
}

export function buildReviewCardContent(payload: ReviewCardPayload): string {
  reviewCardPayloadSchema.parse(payload);
  const labels: Record<ReviewDecision, string> = {
    pending: "⏳ Pending",
    approved: "✅ Approved",
    changes_requested: "✏️ Changes requested",
    rejected: "⛔ Rejected",
  };
  const lines = [
    `## ${payload.title}`,
    `${labels[payload.decision]} — ${payload.patch_ref}`,
  ];
  if (payload.evidence.length > 0) {
    lines.push(`**Evidence:** ${payload.evidence.join("; ")}`);
  }
  if (payload.test_summary) {
    lines.push(`**Tests:** ${payload.test_summary}`);
  }
  if (payload.reviewer) {
    lines.push(`**Reviewer:** ${payload.reviewer}`);
  }
  if (payload.shadow) {
    lines.push("**SHADOW / NOT DELIVERED**");
  }
  return lines.join("\n\n");
}

export function buildTriageCardTags(input: {
  channelId: string;
  payload: TriageCardPayload;
  recipientPubkeys?: string[];
  rootEventId?: string | null;
  parentEventId?: string | null;
}): string[][] {
  const parsed = triageCardPayloadSchema.parse(input.payload);
  const encoded = JSON.stringify(parsed);
  const payloadHash = bytesToHex(sha256(new TextEncoder().encode(encoded)));
  const tags: string[][] = [["h", input.channelId]];
  for (const pubkey of new Set(input.recipientPubkeys ?? [])) {
    if (pubkey) tags.push(["p", pubkey]);
  }
  if (input.rootEventId && input.parentEventId) {
    if (input.rootEventId !== input.parentEventId) {
      tags.push(["e", input.rootEventId, "", "root"]);
    }
    tags.push(["e", input.parentEventId, "", "reply"]);
  }
  tags.push(
    ["triage_card", encoded],
    ["payload_hash", payloadHash],
    ["shadow", parsed.shadow ? "1" : "0"],
  );
  return tags;
}

export function buildReviewCardTags(input: {
  channelId: string;
  payload: ReviewCardPayload;
  recipientPubkeys?: string[];
  rootEventId?: string | null;
  parentEventId?: string | null;
}): string[][] {
  const parsed = reviewCardPayloadSchema.parse(input.payload);
  const encoded = JSON.stringify(parsed);
  const payloadHash = bytesToHex(sha256(new TextEncoder().encode(encoded)));
  const tags: string[][] = [["h", input.channelId]];
  for (const pubkey of new Set(input.recipientPubkeys ?? [])) {
    if (pubkey) tags.push(["p", pubkey]);
  }
  if (input.rootEventId && input.parentEventId) {
    if (input.rootEventId !== input.parentEventId) {
      tags.push(["e", input.rootEventId, "", "root"]);
    }
    tags.push(["e", input.parentEventId, "", "reply"]);
  }
  tags.push(
    ["review_card", encoded],
    ["payload_hash", payloadHash],
    ["shadow", parsed.shadow ? "1" : "0"],
  );
  return tags;
}
