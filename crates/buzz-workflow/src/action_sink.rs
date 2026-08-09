//! Action sink trait — interface for workflow side-effects.
//!
//! The relay implements [`ActionSink`] to provide direct DB access to the
//! executor, replacing the HTTP loopback pattern.

use std::future::Future;
use std::pin::Pin;

use buzz_core::tenant::CommunityId;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Errors from action sink operations.
#[derive(Debug, thiserror::Error)]
pub enum ActionSinkError {
    /// An input parameter is malformed (e.g. invalid UUID).
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// The target channel does not exist.
    #[error("channel not found: {0}")]
    ChannelNotFound(String),
    /// The target channel is archived.
    #[error("channel is archived: {0}")]
    ChannelArchived(String),
    /// Nostr event construction or signing failed.
    #[error("event construction failed: {0}")]
    EventBuild(String),
    /// A database operation failed.
    #[error("database error: {0}")]
    Database(String),
    /// Message content is empty or whitespace-only.
    #[error("empty message content")]
    EmptyContent,
}

/// Data required to publish one durable native approval request.
///
/// Keeping the publication payload together prevents the action-sink contract
/// from growing an argument list whenever the native event gains another
/// field. The raw approval token is intentionally absent; only its hash is
/// allowed across this boundary.
#[derive(Debug, Clone)]
pub struct ApprovalRequestPublication {
    /// The community that owns the workflow run.
    pub community_id: CommunityId,
    /// UUID string of the destination channel.
    pub channel_id: String,
    /// Workflow identity.
    pub workflow_id: Uuid,
    /// Run identity.
    pub run_id: Uuid,
    /// Workflow step identity.
    pub step_id: String,
    /// Zero-based workflow step index.
    pub step_index: i32,
    /// Serialized approver specification.
    pub approver_spec: String,
    /// Human-readable approval message.
    pub message: String,
    /// Approval expiry.
    pub expires_at: DateTime<Utc>,
    /// SHA-256 digest used by the approval row.
    pub token_hash: String,
    /// Triggering event ID, when the approval belongs in a thread.
    pub origin_event_id: Option<String>,
    /// Workflow owner pubkey used for attribution and access checks.
    pub author_pubkey: String,
}

impl From<ActionSinkError> for crate::WorkflowError {
    fn from(e: ActionSinkError) -> Self {
        crate::WorkflowError::WebhookError(e.to_string())
    }
}

/// Interface for workflow actions that produce side effects.
///
/// Implemented by the relay to provide direct DB/event access to the executor.
/// This replaces the HTTP loopback where the executor POSTed to the relay's
/// REST API (which failed with 401 auth errors).
///
/// Returns `Pin<Box<dyn Future>>` for dyn-compatibility — required because
/// `WorkflowEngine` stores `Arc<dyn ActionSink>`.
pub trait ActionSink: Send + Sync {
    /// Post a message to a channel on behalf of a workflow owner.
    ///
    /// - `community_id`: the server-resolved community that owns the workflow
    ///   run driving this side effect. The relay-signed message is published
    ///   under *this* community, never the deployment/default tenant — the run
    ///   carries its owning community so a workflow in community B posts into B
    ///   even though the side effect has no inbound connection to bind.
    /// - `channel_id`: UUID string of the target channel
    /// - `text`: message body (must not be empty/whitespace-only)
    /// - `author_pubkey`: hex-encoded pubkey of the workflow owner (used for
    ///   the `p` attribution tag; the relay keypair signs the event)
    ///
    /// Returns the event ID hex string on success.
    fn send_message(
        &self,
        community_id: CommunityId,
        channel_id: &str,
        text: &str,
        author_pubkey: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ActionSinkError>> + Send + '_>>;

    /// Publish the durable native approval-request event after the workflow
    /// approval row has been committed.
    ///
    /// `token_hash` is the SHA-256 hex digest already used by
    /// `workflow_approvals`; the raw approval token must never cross this
    /// boundary or be included in the event payload. `origin_event_id` is the
    /// triggering message ID when the workflow was started from a channel
    /// event, allowing the relay to place the card in that thread.
    fn publish_approval_request(
        &self,
        request: ApprovalRequestPublication,
    ) -> Pin<Box<dyn Future<Output = Result<String, ActionSinkError>> + Send + '_>>;
}
