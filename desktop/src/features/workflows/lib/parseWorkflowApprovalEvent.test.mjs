import assert from "node:assert/strict";
import test from "node:test";

import { parseWorkflowApprovalEvent } from "./parseWorkflowApprovalEvent.ts";

const approver = "a".repeat(64);
const tokenHash = "b".repeat(64);

test("parses an official native approval request event", () => {
  const approval = parseWorkflowApprovalEvent({
    kind: 46010,
    created_at: 1_754_760_000,
    tags: [
      ["h", "channel-625"],
      ["d", tokenHash],
      ["workflow", "workflow-625"],
      ["run", "run-625"],
      ["step", "gate"],
      ["p", approver],
    ],
    content: JSON.stringify({
      message: "Approve corrected #625 redraft",
      approver_spec: approver,
      step_index: 0,
      expires_at: "2026-08-10T17:34:45.490Z",
    }),
  });

  assert.deepEqual(approval, {
    token: tokenHash,
    workflowId: "workflow-625",
    runId: "run-625",
    stepId: "gate",
    stepIndex: 0,
    message: "Approve corrected #625 redraft",
    approverSpec: approver,
    status: "pending",
    approverPubkey: approver,
    note: null,
    expiresAt: "2026-08-10T17:34:45.490Z",
    createdAt: 1_754_760_000,
  });
});

test("rejects malformed or non-approval events", () => {
  assert.equal(
    parseWorkflowApprovalEvent({ kind: 9, tags: [], content: "hello" }),
    null,
  );
  assert.equal(
    parseWorkflowApprovalEvent({
      kind: 46010,
      tags: [["d", tokenHash]],
      content: "{}",
    }),
    null,
  );
  assert.equal(
    parseWorkflowApprovalEvent({
      kind: 46010,
      tags: [
        ["d", tokenHash],
        ["workflow", "workflow-625"],
        ["run", "run-625"],
        ["step", "gate"],
      ],
      content: "not-json",
    }),
    null,
  );
});
