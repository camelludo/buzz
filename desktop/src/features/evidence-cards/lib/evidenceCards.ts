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

const hex64Schema = z.string().regex(/^[0-9a-f]{64}$/i);

const boundedText = (max: number) => z.string().trim().min(1).max(max);

const evidencePacketPayloadSchema = z.object({
  schema_version: z.literal(1),
  packet_id: z.string().uuid(),
  title: boundedText(160),
  source_url: httpUrlSchema,
  source_hash: hex64Schema.optional(),
  parse_summary: boundedText(2_000),
  confidence: z.number().min(0).max(1),
  red_flags: z.array(boundedText(500)).max(16),
  missing: z.array(boundedText(500)).max(16),
  recommendation: boundedText(2_000),
  record_url: httpUrlSchema.optional(),
  shadow: z.boolean(),
});

export const DELIVERY_RECEIPT_STATUSES = [
  "sent",
  "delivered",
  "failed",
  "proof_missing",
] as const;

export type DeliveryReceiptStatus = (typeof DELIVERY_RECEIPT_STATUSES)[number];

const deliveryReceiptPayloadSchema = z.object({
  schema_version: z.literal(1),
  receipt_id: z.string().uuid(),
  action_id: z.string().uuid(),
  status: z.enum(DELIVERY_RECEIPT_STATUSES),
  detail: boundedText(2_000),
  proof_url: httpUrlSchema.optional(),
  shadow: z.boolean(),
});

export type EvidencePacketPayload = z.infer<typeof evidencePacketPayloadSchema>;
export type DeliveryReceiptPayload = z.infer<
  typeof deliveryReceiptPayloadSchema
>;

export type ParsedEvidencePacket = {
  payload: EvidencePacketPayload;
  payloadHash: string;
};

export type ParsedDeliveryReceipt = {
  payload: DeliveryReceiptPayload;
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

export function parseEvidencePacket(
  tags: string[][],
): ParsedEvidencePacket | null {
  const encoded = findTag(tags, "evidence_packet");
  const payloadHash = findTag(tags, "payload_hash");
  if (!encoded) return null;

  const verifiedHash = verifyPayloadHash(encoded, payloadHash);
  if (!verifiedHash) return null;

  try {
    const payload = evidencePacketPayloadSchema.parse(JSON.parse(encoded));
    return { payload, payloadHash: verifiedHash };
  } catch {
    return null;
  }
}

export function parseDeliveryReceipt(
  tags: string[][],
): ParsedDeliveryReceipt | null {
  const encoded = findTag(tags, "delivery_receipt");
  const payloadHash = findTag(tags, "payload_hash");
  if (!encoded) return null;

  const verifiedHash = verifyPayloadHash(encoded, payloadHash);
  if (!verifiedHash) return null;

  try {
    const payload = deliveryReceiptPayloadSchema.parse(JSON.parse(encoded));
    return { payload, payloadHash: verifiedHash };
  } catch {
    return null;
  }
}

export function buildEvidencePacketContent(
  payload: EvidencePacketPayload,
): string {
  evidencePacketPayloadSchema.parse(payload);
  const lines = [
    `## ${payload.title}`,
    payload.parse_summary,
    `**Confidence:** ${Math.round(payload.confidence * 100)}%`,
    `**Recommendation:** ${payload.recommendation}`,
  ];
  if (payload.red_flags.length > 0) {
    lines.push(`**Red flags:** ${payload.red_flags.join("; ")}`);
  }
  if (payload.missing.length > 0) {
    lines.push(`**Missing:** ${payload.missing.join("; ")}`);
  }
  if (payload.shadow) {
    lines.push("**SHADOW / NOT DELIVERED**");
  }
  return lines.join("\n\n");
}

export function buildDeliveryReceiptContent(
  payload: DeliveryReceiptPayload,
): string {
  deliveryReceiptPayloadSchema.parse(payload);
  const labels: Record<DeliveryReceiptStatus, string> = {
    sent: "📤 Sent",
    delivered: "✅ Delivered",
    failed: "⛔ Failed",
    proof_missing: "⚠️ Proof missing",
  };
  const base = `${labels[payload.status]} — ${payload.detail}`;
  return payload.shadow ? `${base}\n\n**SHADOW / NOT DELIVERED**` : base;
}

export function buildEvidencePacketTags(input: {
  channelId: string;
  payload: EvidencePacketPayload;
  recipientPubkeys?: string[];
  rootEventId?: string | null;
  parentEventId?: string | null;
}): string[][] {
  const parsed = evidencePacketPayloadSchema.parse(input.payload);
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
    ["evidence_packet", encoded],
    ["payload_hash", payloadHash],
    ["shadow", parsed.shadow ? "1" : "0"],
  );
  return tags;
}

export function buildDeliveryReceiptTags(input: {
  channelId: string;
  payload: DeliveryReceiptPayload;
  recipientPubkeys?: string[];
  rootEventId?: string | null;
  parentEventId?: string | null;
}): string[][] {
  const parsed = deliveryReceiptPayloadSchema.parse(input.payload);
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
    ["delivery_receipt", encoded],
    ["payload_hash", payloadHash],
    ["shadow", parsed.shadow ? "1" : "0"],
  );
  return tags;
}
