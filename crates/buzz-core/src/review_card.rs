//! Typed payloads for channel-native review cards.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Current wire schema for review-card payloads.
pub const REVIEW_CARD_SCHEMA_VERSION: u8 = 1;

/// Decision state recorded on a review card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    /// Awaiting reviewer action.
    Pending,
    /// Reviewer approved the change.
    Approved,
    /// Reviewer requested changes.
    ChangesRequested,
    /// Reviewer rejected the change.
    Rejected,
}

/// Structured data carried by a `kind:40014` review-card event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCardPayload {
    /// Payload schema version.
    pub schema_version: u8,
    /// Stable business-level review identifier.
    pub review_id: Uuid,
    /// Short review title.
    pub title: String,
    /// PR / commit / issue link or ref.
    pub patch_ref: String,
    /// Evidence bullets supporting the review state.
    pub evidence: Vec<String>,
    /// Optional condensed test summary.
    pub test_summary: Option<String>,
    /// Current review decision.
    pub decision: ReviewDecision,
    /// Optional reviewer identity label.
    pub reviewer: Option<String>,
    /// Optional authoritative-record URL.
    pub record_url: Option<String>,
    /// Whether this card is explicitly non-production.
    pub shadow: bool,
}

impl ReviewCardPayload {
    /// Validate the bounded wire contract.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != REVIEW_CARD_SCHEMA_VERSION {
            return Err("unsupported review card schema version");
        }
        if self.title.trim().is_empty() || self.patch_ref.trim().is_empty() {
            return Err("review card text fields must not be empty");
        }
        if self.title.len() > 160 {
            return Err("review card text field exceeds its size limit");
        }
        if self.patch_ref.len() > 2_048 {
            return Err("review card patch_ref exceeds its size limit");
        }
        if self.evidence.len() > 16 {
            return Err("review card evidence items exceed the maximum count");
        }
        for item in &self.evidence {
            if item.trim().is_empty() || item.len() > 500 {
                return Err("review card evidence item must be non-empty and at most 500 bytes");
            }
        }
        if let Some(test_summary) = &self.test_summary {
            if test_summary.trim().is_empty() || test_summary.len() > 2_000 {
                return Err(
                    "review card test_summary must be non-empty and at most 2000 bytes when set",
                );
            }
        }
        if let Some(reviewer) = &self.reviewer {
            if reviewer.trim().is_empty() || reviewer.len() > 160 {
                return Err(
                    "review card reviewer must be non-empty and at most 160 bytes when set",
                );
            }
        }
        if self
            .record_url
            .as_ref()
            .is_some_and(|record_url| record_url.len() > 2_048)
        {
            return Err("review card record URL exceeds its size limit");
        }
        if let Some(record_url) = &self.record_url {
            let parsed = url::Url::parse(record_url)
                .map_err(|_| "review card record URL must be an absolute HTTP(S) URL")?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("review card record URL must be an absolute HTTP(S) URL");
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

    fn sample_payload() -> ReviewCardPayload {
        ReviewCardPayload {
            schema_version: REVIEW_CARD_SCHEMA_VERSION,
            review_id: Uuid::parse_str("55555555-5555-4555-8555-555555555555").unwrap(),
            title: "Evidence card suite slice 3".into(),
            patch_ref: "https://github.com/block/buzz/pull/999".into(),
            evidence: vec![
                "Unit tests cover hash stability and enum serde.".into(),
                "Desktop parse path fails closed on bad hashes.".into(),
            ],
            test_summary: Some("cargo test -p buzz-core -p buzz-sdk: ok".into()),
            decision: ReviewDecision::Pending,
            reviewer: Some("tyler".into()),
            record_url: Some("https://stomaton.example/reviews/999".into()),
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
    fn decision_enum_serde_uses_snake_case() {
        let json = serde_json::to_string(&ReviewDecision::ChangesRequested).unwrap();
        assert_eq!(json, "\"changes_requested\"");

        let decoded: ReviewDecision = serde_json::from_str("\"approved\"").unwrap();
        assert_eq!(decoded, ReviewDecision::Approved);

        let round_trip = sample_payload();
        let encoded = serde_json::to_string(&round_trip).unwrap();
        let restored: ReviewCardPayload = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, round_trip);
        assert_eq!(restored.decision, ReviewDecision::Pending);
    }

    #[test]
    fn rejects_empty_title_and_oversized_evidence() {
        let mut empty_title = sample_payload();
        empty_title.title = "   ".into();
        assert_eq!(
            empty_title.validate(),
            Err("review card text fields must not be empty")
        );

        let mut too_many = sample_payload();
        too_many.evidence = (0..17).map(|i| format!("item {i}")).collect();
        assert_eq!(
            too_many.validate(),
            Err("review card evidence items exceed the maximum count")
        );

        assert!(sample_payload().validate().is_ok());
    }
}
