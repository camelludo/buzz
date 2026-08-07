//! Typed payloads for channel-native triage cards.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Current wire schema for triage-card payloads.
pub const TRIAGE_CARD_SCHEMA_VERSION: u8 = 1;

/// Risk band for a triage group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageRisk {
    /// Low-urgency items.
    Low,
    /// Medium-urgency items.
    Medium,
    /// High-urgency items.
    High,
}

/// One group inside a triage card (risk/age/channel bucket).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriageGroup {
    /// Human-readable group label.
    pub label: String,
    /// Item count in this group.
    pub count: u32,
    /// Optional age of the oldest item (e.g. `"2d"`, `"4h"`).
    pub oldest_age: Option<String>,
    /// Risk band for this group.
    pub risk: TriageRisk,
}

/// Structured data carried by a `kind:40013` triage-card event.
///
/// Semantics: one grouped triage view (e.g. 68 drafts grouped by risk/age/channel)
/// — never N separate action buttons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriageCardPayload {
    /// Payload schema version.
    pub schema_version: u8,
    /// Stable business-level triage identifier.
    pub triage_id: Uuid,
    /// Short triage title.
    pub title: String,
    /// Grouped buckets (label + count + age + risk).
    pub groups: Vec<TriageGroup>,
    /// Single proposed next action for the whole triage set.
    pub proposed_action: String,
    /// Optional authoritative-record URL.
    pub record_url: Option<String>,
    /// Whether this card is explicitly non-production.
    pub shadow: bool,
}

impl TriageCardPayload {
    /// Validate the bounded wire contract.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != TRIAGE_CARD_SCHEMA_VERSION {
            return Err("unsupported triage card schema version");
        }
        if self.title.trim().is_empty() || self.proposed_action.trim().is_empty() {
            return Err("triage card text fields must not be empty");
        }
        if self.title.len() > 160 || self.proposed_action.len() > 2_000 {
            return Err("triage card text field exceeds its size limit");
        }
        if self.groups.is_empty() || self.groups.len() > 32 {
            return Err("triage card must expose between one and 32 groups");
        }
        for group in &self.groups {
            if group.label.trim().is_empty() || group.label.len() > 160 {
                return Err("triage group label must be non-empty and at most 160 bytes");
            }
            if group.count == 0 {
                return Err("triage group count must be greater than zero");
            }
            if let Some(oldest_age) = &group.oldest_age {
                if oldest_age.trim().is_empty() || oldest_age.len() > 64 {
                    return Err(
                        "triage group oldest_age must be non-empty and at most 64 bytes when set",
                    );
                }
            }
        }
        if self
            .record_url
            .as_ref()
            .is_some_and(|record_url| record_url.len() > 2_048)
        {
            return Err("triage card record URL exceeds its size limit");
        }
        if let Some(record_url) = &self.record_url {
            let parsed = url::Url::parse(record_url)
                .map_err(|_| "triage card record URL must be an absolute HTTP(S) URL")?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("triage card record URL must be an absolute HTTP(S) URL");
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

    fn sample_payload() -> TriageCardPayload {
        TriageCardPayload {
            schema_version: TRIAGE_CARD_SCHEMA_VERSION,
            triage_id: Uuid::parse_str("44444444-4444-4444-8444-444444444444").unwrap(),
            title: "68 shadow drafts need triage".into(),
            groups: vec![
                TriageGroup {
                    label: "High risk · >7d".into(),
                    count: 12,
                    oldest_age: Some("14d".into()),
                    risk: TriageRisk::High,
                },
                TriageGroup {
                    label: "Medium risk · sales".into(),
                    count: 41,
                    oldest_age: Some("3d".into()),
                    risk: TriageRisk::Medium,
                },
                TriageGroup {
                    label: "Low risk · ops".into(),
                    count: 15,
                    oldest_age: None,
                    risk: TriageRisk::Low,
                },
            ],
            proposed_action: "Open the oldest high-risk group first.".into(),
            record_url: Some("https://stomaton.example/triage/68".into()),
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
    fn risk_enum_serde_uses_snake_case() {
        let json = serde_json::to_string(&TriageRisk::High).unwrap();
        assert_eq!(json, "\"high\"");

        let decoded: TriageRisk = serde_json::from_str("\"medium\"").unwrap();
        assert_eq!(decoded, TriageRisk::Medium);

        let round_trip = sample_payload();
        let encoded = serde_json::to_string(&round_trip).unwrap();
        let restored: TriageCardPayload = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, round_trip);
        assert_eq!(restored.groups[0].risk, TriageRisk::High);
    }

    #[test]
    fn rejects_empty_groups_and_zero_counts() {
        let mut empty = sample_payload();
        empty.groups.clear();
        assert_eq!(
            empty.validate(),
            Err("triage card must expose between one and 32 groups")
        );

        let mut zero = sample_payload();
        zero.groups[0].count = 0;
        assert_eq!(
            zero.validate(),
            Err("triage group count must be greater than zero")
        );

        assert!(sample_payload().validate().is_ok());
    }
}
