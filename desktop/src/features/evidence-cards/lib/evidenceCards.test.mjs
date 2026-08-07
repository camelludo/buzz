import assert from "node:assert/strict";
import test from "node:test";

import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex } from "@noble/hashes/utils.js";

import {
  buildDeliveryReceiptContent,
  buildDeliveryReceiptTags,
  buildEvidencePacketContent,
  buildEvidencePacketTags,
  parseDeliveryReceipt,
  parseEvidencePacket,
} from "./evidenceCards.ts";

const CHANNEL_ID = "36411e44-0e2d-4cfe-bd6e-567eb169db9f";

const packet = {
  schema_version: 1,
  packet_id: "11111111-1111-4111-8111-111111111111",
  title: "Booking confirmation evidence",
  source_url: "https://stomaton.example/evidence/625",
  source_hash: "ab".repeat(32),
  parse_summary: "PDF contains a confirmed sailing window.",
  confidence: 0.91,
  red_flags: ["Consignee address is incomplete."],
  missing: ["Bill of lading number"],
  recommendation: "Request the missing BL number before dispatch.",
  record_url: "https://stomaton.example/cases/625",
  shadow: true,
};

const receipt = {
  schema_version: 1,
  receipt_id: "22222222-2222-4222-8222-222222222222",
  action_id: "33333333-3333-4333-8333-333333333333",
  status: "delivered",
  detail: "Shadow WhatsApp delivery confirmed by provider webhook.",
  proof_url: "https://stomaton.example/proofs/abc",
  shadow: true,
};

const ENCODED_PACKET = JSON.stringify(packet);
const PACKET_HASH = bytesToHex(
  sha256(new TextEncoder().encode(ENCODED_PACKET)),
);

const ENCODED_RECEIPT = JSON.stringify(receipt);
const RECEIPT_HASH = bytesToHex(
  sha256(new TextEncoder().encode(ENCODED_RECEIPT)),
);

function packetTags(payload) {
  const encoded = JSON.stringify(payload);
  const hash = bytesToHex(sha256(new TextEncoder().encode(encoded)));
  return [
    ["evidence_packet", encoded],
    ["payload_hash", hash],
  ];
}

function receiptTags(payload) {
  const encoded = JSON.stringify(payload);
  const hash = bytesToHex(sha256(new TextEncoder().encode(encoded)));
  return [
    ["delivery_receipt", encoded],
    ["payload_hash", hash],
  ];
}

test("parses a versioned evidence packet tag without consuming its Markdown fallback", () => {
  const parsed = parseEvidencePacket([
    ["h", CHANNEL_ID],
    ["evidence_packet", ENCODED_PACKET],
    ["payload_hash", PACKET_HASH],
    ["shadow", "1"],
  ]);

  assert.deepEqual(parsed, { payload: packet, payloadHash: PACKET_HASH });
});

test("rejects an evidence packet whose payload hash does not match", () => {
  assert.equal(
    parseEvidencePacket([
      ["evidence_packet", ENCODED_PACKET],
      ["payload_hash", "c".repeat(64)],
    ]),
    null,
  );
});

test("fails closed for malformed evidence packets and missing fields", () => {
  assert.equal(parseEvidencePacket([["evidence_packet", "not-json"]]), null);
  assert.equal(parseEvidencePacket(packetTags({ ...packet, title: "" })), null);
  assert.equal(
    parseEvidencePacket(packetTags({ ...packet, confidence: 1.5 })),
    null,
  );
  assert.equal(
    parseEvidencePacket(
      packetTags({ ...packet, source_url: "javascript:alert(1)" }),
    ),
    null,
  );
  assert.equal(
    parseEvidencePacket(
      packetTags({
        ...packet,
        record_url: "ftp://example.com/file",
      }),
    ),
    null,
  );
  assert.equal(
    parseEvidencePacket([
      ["evidence_packet", ENCODED_PACKET],
      // missing payload_hash
    ]),
    null,
  );
});

test("builds a native evidence packet envelope with a matching payload hash", () => {
  const tags = buildEvidencePacketTags({
    channelId: CHANNEL_ID,
    payload: packet,
  });

  assert.deepEqual(tags[0], ["h", CHANNEL_ID]);
  assert.equal(tags.find(([name]) => name === "shadow")?.[1], "1");
  const encoded = tags.find(([name]) => name === "evidence_packet")?.[1];
  const payloadHash = tags.find(([name]) => name === "payload_hash")?.[1];
  assert.equal(encoded, ENCODED_PACKET);
  assert.equal(payloadHash, PACKET_HASH);
  assert.deepEqual(parseEvidencePacket(tags), {
    payload: packet,
    payloadHash: PACKET_HASH,
  });
  assert.match(buildEvidencePacketContent(packet), /SHADOW \/ NOT DELIVERED/);
});

test("parses a versioned delivery receipt tag", () => {
  const parsed = parseDeliveryReceipt([
    ["h", CHANNEL_ID],
    ["delivery_receipt", ENCODED_RECEIPT],
    ["payload_hash", RECEIPT_HASH],
    ["shadow", "1"],
  ]);

  assert.deepEqual(parsed, { payload: receipt, payloadHash: RECEIPT_HASH });
});

test("rejects a delivery receipt whose payload hash does not match", () => {
  assert.equal(
    parseDeliveryReceipt([
      ["delivery_receipt", ENCODED_RECEIPT],
      ["payload_hash", "d".repeat(64)],
    ]),
    null,
  );
});

test("fails closed for malformed delivery receipts and unsupported status", () => {
  assert.equal(parseDeliveryReceipt([["delivery_receipt", "not-json"]]), null);
  assert.equal(
    parseDeliveryReceipt(receiptTags({ ...receipt, status: "bounced" })),
    null,
  );
  assert.equal(
    parseDeliveryReceipt(receiptTags({ ...receipt, detail: "" })),
    null,
  );
  assert.equal(
    parseDeliveryReceipt(
      receiptTags({ ...receipt, proof_url: "javascript:alert(1)" }),
    ),
    null,
  );
  assert.equal(
    parseDeliveryReceipt([
      ["delivery_receipt", ENCODED_RECEIPT],
      // missing payload_hash
    ]),
    null,
  );
});

test("builds a native delivery receipt envelope with a matching payload hash", () => {
  const tags = buildDeliveryReceiptTags({
    channelId: CHANNEL_ID,
    payload: receipt,
  });

  assert.deepEqual(tags[0], ["h", CHANNEL_ID]);
  assert.equal(tags.find(([name]) => name === "shadow")?.[1], "1");
  const encoded = tags.find(([name]) => name === "delivery_receipt")?.[1];
  const payloadHash = tags.find(([name]) => name === "payload_hash")?.[1];
  assert.equal(encoded, ENCODED_RECEIPT);
  assert.equal(payloadHash, RECEIPT_HASH);
  assert.deepEqual(parseDeliveryReceipt(tags), {
    payload: receipt,
    payloadHash: RECEIPT_HASH,
  });
  assert.match(buildDeliveryReceiptContent(receipt), /SHADOW \/ NOT DELIVERED/);
});

test("adds DM recipients so the relay can route a native evidence packet", () => {
  const tags = buildEvidencePacketTags({
    channelId: CHANNEL_ID,
    payload: packet,
    recipientPubkeys: ["c".repeat(64), "c".repeat(64)],
  });

  assert.deepEqual(tags.slice(0, 2), [
    ["h", CHANNEL_ID],
    ["p", "c".repeat(64)],
  ]);
  assert.equal(tags.filter(([name]) => name === "p").length, 1);
});
