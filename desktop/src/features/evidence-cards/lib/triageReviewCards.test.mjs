import assert from "node:assert/strict";
import test from "node:test";

import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex } from "@noble/hashes/utils.js";

import {
  buildReviewCardContent,
  buildReviewCardTags,
  buildTriageCardContent,
  buildTriageCardTags,
  parseReviewCard,
  parseTriageCard,
} from "./triageReviewCards.ts";

const CHANNEL_ID = "36411e44-0e2d-4cfe-bd6e-567eb169db9f";

const triage = {
  schema_version: 1,
  triage_id: "44444444-4444-4444-8444-444444444444",
  title: "68 shadow drafts need triage",
  groups: [
    {
      label: "High risk · >7d",
      count: 12,
      oldest_age: "14d",
      risk: "high",
    },
    {
      label: "Medium risk · sales",
      count: 41,
      oldest_age: "3d",
      risk: "medium",
    },
    {
      label: "Low risk · ops",
      count: 15,
      risk: "low",
    },
  ],
  proposed_action: "Open the oldest high-risk group first.",
  record_url: "https://stomaton.example/triage/68",
  shadow: true,
};

const review = {
  schema_version: 1,
  review_id: "55555555-5555-4555-8555-555555555555",
  title: "Evidence card suite slice 3",
  patch_ref: "https://github.com/block/buzz/pull/999",
  evidence: [
    "Unit tests cover hash stability and enum serde.",
    "Desktop parse path fails closed on bad hashes.",
  ],
  test_summary: "cargo test -p buzz-core -p buzz-sdk: ok",
  decision: "pending",
  reviewer: "tyler",
  record_url: "https://stomaton.example/reviews/999",
  shadow: true,
};

const ENCODED_TRIAGE = JSON.stringify(triage);
const TRIAGE_HASH = bytesToHex(
  sha256(new TextEncoder().encode(ENCODED_TRIAGE)),
);

const ENCODED_REVIEW = JSON.stringify(review);
const REVIEW_HASH = bytesToHex(
  sha256(new TextEncoder().encode(ENCODED_REVIEW)),
);

function triageTags(payload) {
  const encoded = JSON.stringify(payload);
  const hash = bytesToHex(sha256(new TextEncoder().encode(encoded)));
  return [
    ["triage_card", encoded],
    ["payload_hash", hash],
  ];
}

function reviewTags(payload) {
  const encoded = JSON.stringify(payload);
  const hash = bytesToHex(sha256(new TextEncoder().encode(encoded)));
  return [
    ["review_card", encoded],
    ["payload_hash", hash],
  ];
}

test("parses a versioned triage card tag without consuming its Markdown fallback", () => {
  const parsed = parseTriageCard([
    ["h", CHANNEL_ID],
    ["triage_card", ENCODED_TRIAGE],
    ["payload_hash", TRIAGE_HASH],
    ["shadow", "1"],
  ]);

  assert.deepEqual(parsed, { payload: triage, payloadHash: TRIAGE_HASH });
});

test("rejects a triage card whose payload hash does not match", () => {
  assert.equal(
    parseTriageCard([
      ["triage_card", ENCODED_TRIAGE],
      ["payload_hash", "c".repeat(64)],
    ]),
    null,
  );
});

test("fails closed for malformed triage cards and invalid groups", () => {
  assert.equal(parseTriageCard([["triage_card", "not-json"]]), null);
  assert.equal(parseTriageCard(triageTags({ ...triage, title: "" })), null);
  assert.equal(
    parseTriageCard(triageTags({ ...triage, groups: [] })),
    null,
  );
  assert.equal(
    parseTriageCard(
      triageTags({
        ...triage,
        groups: [{ label: "x", count: 0, risk: "low" }],
      }),
    ),
    null,
  );
  assert.equal(
    parseTriageCard(
      triageTags({
        ...triage,
        groups: [{ label: "x", count: 1, risk: "critical" }],
      }),
    ),
    null,
  );
  assert.equal(
    parseTriageCard(
      triageTags({
        ...triage,
        record_url: "ftp://example.com/file",
      }),
    ),
    null,
  );
  assert.equal(
    parseTriageCard([
      ["triage_card", ENCODED_TRIAGE],
      // missing payload_hash
    ]),
    null,
  );
});

test("builds a native triage card envelope with a matching payload hash", () => {
  const tags = buildTriageCardTags({
    channelId: CHANNEL_ID,
    payload: triage,
  });

  assert.deepEqual(tags[0], ["h", CHANNEL_ID]);
  assert.equal(tags.find(([name]) => name === "shadow")?.[1], "1");
  const encoded = tags.find(([name]) => name === "triage_card")?.[1];
  const payloadHash = tags.find(([name]) => name === "payload_hash")?.[1];
  assert.equal(encoded, ENCODED_TRIAGE);
  assert.equal(payloadHash, TRIAGE_HASH);
  assert.deepEqual(parseTriageCard(tags), {
    payload: triage,
    payloadHash: TRIAGE_HASH,
  });
  assert.match(buildTriageCardContent(triage), /SHADOW \/ NOT DELIVERED/);
});

test("parses a versioned review card tag", () => {
  const parsed = parseReviewCard([
    ["h", CHANNEL_ID],
    ["review_card", ENCODED_REVIEW],
    ["payload_hash", REVIEW_HASH],
    ["shadow", "1"],
  ]);

  assert.deepEqual(parsed, { payload: review, payloadHash: REVIEW_HASH });
});

test("rejects a review card whose payload hash does not match", () => {
  assert.equal(
    parseReviewCard([
      ["review_card", ENCODED_REVIEW],
      ["payload_hash", "d".repeat(64)],
    ]),
    null,
  );
});

test("fails closed for malformed review cards and unsupported decision", () => {
  assert.equal(parseReviewCard([["review_card", "not-json"]]), null);
  assert.equal(
    parseReviewCard(reviewTags({ ...review, decision: "lgtm" })),
    null,
  );
  assert.equal(
    parseReviewCard(reviewTags({ ...review, title: "" })),
    null,
  );
  assert.equal(
    parseReviewCard(reviewTags({ ...review, patch_ref: "" })),
    null,
  );
  assert.equal(
    parseReviewCard(
      reviewTags({ ...review, record_url: "javascript:alert(1)" }),
    ),
    null,
  );
  assert.equal(
    parseReviewCard([
      ["review_card", ENCODED_REVIEW],
      // missing payload_hash
    ]),
    null,
  );
});

test("builds a native review card envelope with a matching payload hash", () => {
  const tags = buildReviewCardTags({
    channelId: CHANNEL_ID,
    payload: review,
  });

  assert.deepEqual(tags[0], ["h", CHANNEL_ID]);
  assert.equal(tags.find(([name]) => name === "shadow")?.[1], "1");
  const encoded = tags.find(([name]) => name === "review_card")?.[1];
  const payloadHash = tags.find(([name]) => name === "payload_hash")?.[1];
  assert.equal(encoded, ENCODED_REVIEW);
  assert.equal(payloadHash, REVIEW_HASH);
  assert.deepEqual(parseReviewCard(tags), {
    payload: review,
    payloadHash: REVIEW_HASH,
  });
  assert.match(buildReviewCardContent(review), /SHADOW \/ NOT DELIVERED/);
});

test("adds DM recipients so the relay can route a native triage card", () => {
  const tags = buildTriageCardTags({
    channelId: CHANNEL_ID,
    payload: triage,
    recipientPubkeys: ["c".repeat(64), "c".repeat(64)],
  });

  assert.deepEqual(tags.slice(0, 2), [
    ["h", CHANNEL_ID],
    ["p", "c".repeat(64)],
  ]);
  assert.equal(tags.filter(([name]) => name === "p").length, 1);
});
