const KIND_APPROVAL_REQUEST = 46010;

type ApprovalRequestEvent = {
  kind: number;
  created_at: number;
  tags?: string[][];
  content: string;
};

export type ParsedWorkflowApproval = {
  token: string;
  workflowId: string;
  runId: string;
  stepId: string;
  stepIndex: number;
  message?: string;
  approverSpec: string;
  status: "pending";
  approverPubkey: string | null;
  note: null;
  expiresAt: string;
  createdAt: number;
};

function tagValue(tags: string[][], name: string) {
  return tags.find((tag) => tag[0] === name)?.[1] ?? null;
}

/**
 * Turn the relay's official kind:46010 event into the existing native
 * WorkflowApproval model. The d-tag is the stored SHA-256 token hash; the
 * raw approval token is never sent to the client.
 */
export function parseWorkflowApprovalEvent(
  event: ApprovalRequestEvent,
): ParsedWorkflowApproval | null {
  if (event.kind !== KIND_APPROVAL_REQUEST) {
    return null;
  }

  const tags = event.tags ?? [];
  const token = tagValue(tags, "d");
  const workflowId = tagValue(tags, "workflow");
  const runId = tagValue(tags, "run");
  const stepId = tagValue(tags, "step");
  if (!token || !workflowId || !runId || !stepId) {
    return null;
  }

  let payload: unknown;
  try {
    payload = JSON.parse(event.content);
  } catch {
    return null;
  }

  if (
    !payload ||
    typeof payload !== "object" ||
    typeof (payload as { expires_at?: unknown }).expires_at !== "string" ||
    typeof (payload as { approver_spec?: unknown }).approver_spec !== "string"
  ) {
    return null;
  }

  const typedPayload = payload as {
    message?: unknown;
    approver_spec: string;
    step_index?: unknown;
    expires_at: string;
  };
  const stepIndex =
    typeof typedPayload.step_index === "number" &&
    Number.isInteger(typedPayload.step_index)
      ? typedPayload.step_index
      : 0;
  const approverPubkey =
    tags.find((tag) => tag[0] === "p" && typeof tag[1] === "string")?.[1] ??
    null;

  return {
    token,
    workflowId,
    runId,
    stepId,
    stepIndex,
    message:
      typeof typedPayload.message === "string"
        ? typedPayload.message
        : undefined,
    approverSpec: typedPayload.approver_spec,
    status: "pending",
    approverPubkey,
    note: null,
    expiresAt: typedPayload.expires_at,
    createdAt: event.created_at,
  };
}
