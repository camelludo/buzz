//! Typed payloads for channel-native delivery receipt cards.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Current wire schema for delivery-receipt payloads.
pub const DELIVERY_RECEIPT_SCHEMA_VERSION: u8 = 1;

/// Delivery outcome recorded by a receipt card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryReceiptStatus {
    /// Outbound action was accepted for send.
    Sent,
    /// Downstream system confirmed delivery.
    Delivered,
    /// Delivery attempt failed.
    Failed,
    /// Delivery cannot be proven from available evidence.
    ProofMissing,
}

/// Structured data carried by a `kind:40012` delivery-receipt event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReceiptPayload {
    /// Payload schema version.
    pub schema_version: u8,
    /// Stable business-level receipt identifier.
    pub receipt_id: Uuid,
    /// Idempotency / action identifier this receipt closes.
    pub action_id: Uuid,
    /// Delivery outcome.
    pub status: DeliveryReceiptStatus,
    /// Human-readable detail about the outcome.
    pub detail: String,
    /// Optional proof URL (tracker, provider receipt, etc.).
    pub proof_url: Option<String>,
    /// Whether this receipt is explicitly non-production.
    pub shadow: bool,
}

impl DeliveryReceiptPayload {
    /// Validate the bounded wire contract.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != DELIVERY_RECEIPT_SCHEMA_VERSION {
            return Err("unsupported delivery receipt schema version");
        }
        if self.detail.trim().is_empty() {
            return Err("delivery receipt detail must not be empty");
        }
        if self.detail.len() > 2_000 {
            return Err("delivery receipt detail exceeds its size limit");
        }
        if self
            .proof_url
            .as_ref()
            .is_some_and(|proof_url| proof_url.len() > 2_048)
        {
            return Err("delivery receipt proof URL exceeds its size limit");
        }
        if let Some(proof_url) = &self.proof_url {
            let parsed = url::Url::parse(proof_url)
                .map_err(|_| "delivery receipt proof URL must be an absolute HTTP(S) URL")?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("delivery receipt proof URL must be an absolute HTTP(S) URL");
            }
        }
        Ok(())
    }

    /// Serialize the payload in its canonical field order.
    pub fn canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// SHA-256 digest of the canonical structured payload.
    pub fn payload_hash(&self) -> Result<String, serde_json::Error> {
        let encoded = self.canonical_json()?;
        Ok(hex::encode(Sha256::digest(encoded.as_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> DeliveryReceiptPayload {
        DeliveryReceiptPayload {
            schema_version: DELIVERY_RECEIPT_SCHEMA_VERSION,
            receipt_id: Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
            action_id: Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap(),
            status: DeliveryReceiptStatus::Delivered,
            detail: "Shadow WhatsApp delivery confirmed by provider webhook.".into(),
            proof_url: Some("https://stomaton.example/proofs/abc".into()),
            shadow: true,
        }
    }

    #[test]
    fn payload_hash_is_stable_for_identical_payload() {
        let payload = sample_payload();
        let first = payload.payload_hash().expect("hash");
        let second = payload.payload_hash().expect("hash");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn status_enum_serde_uses_snake_case() {
        let json = serde_json::to_string(&DeliveryReceiptStatus::ProofMissing).unwrap();
        assert_eq!(json, "\"proof_missing\"");

        let decoded: DeliveryReceiptStatus = serde_json::from_str("\"failed\"").unwrap();
        assert_eq!(decoded, DeliveryReceiptStatus::Failed);

        let round_trip = sample_payload();
        let encoded = serde_json::to_string(&round_trip).unwrap();
        let restored: DeliveryReceiptPayload = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, round_trip);
        assert_eq!(restored.status, DeliveryReceiptStatus::Delivered);
    }
}
