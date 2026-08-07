//! Typed payloads for channel-native evidence packet cards.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Current wire schema for evidence-packet payloads.
pub const EVIDENCE_PACKET_SCHEMA_VERSION: u8 = 1;

/// Structured data carried by a `kind:40011` evidence-packet event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidencePacketPayload {
    /// Payload schema version.
    pub schema_version: u8,
    /// Stable business-level packet identifier.
    pub packet_id: Uuid,
    /// Short evidence title.
    pub title: String,
    /// Absolute URL of the evidence source.
    pub source_url: String,
    /// Optional content digest of the fetched source.
    pub source_hash: Option<String>,
    /// Concise parse summary of the evidence.
    pub parse_summary: String,
    /// Model/source confidence in the range `[0.0, 1.0]`.
    pub confidence: f64,
    /// Material concerns raised by the packet.
    pub red_flags: Vec<String>,
    /// Known gaps or missing evidence.
    pub missing: Vec<String>,
    /// Recommended next action given the evidence.
    pub recommendation: String,
    /// Optional authoritative-record URL.
    pub record_url: Option<String>,
    /// Whether this packet is explicitly non-production.
    pub shadow: bool,
}

impl EvidencePacketPayload {
    /// Validate the bounded wire contract.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != EVIDENCE_PACKET_SCHEMA_VERSION {
            return Err("unsupported evidence packet schema version");
        }
        if self.title.trim().is_empty()
            || self.source_url.trim().is_empty()
            || self.parse_summary.trim().is_empty()
            || self.recommendation.trim().is_empty()
        {
            return Err("evidence packet text fields must not be empty");
        }
        if self.title.len() > 160
            || self.parse_summary.len() > 2_000
            || self.recommendation.len() > 2_000
        {
            return Err("evidence packet text field exceeds its size limit");
        }
        if self.source_url.len() > 2_048 {
            return Err("evidence packet source URL exceeds its size limit");
        }
        let source = url::Url::parse(&self.source_url)
            .map_err(|_| "evidence packet source URL must be an absolute HTTP(S) URL")?;
        if !matches!(source.scheme(), "http" | "https") {
            return Err("evidence packet source URL must be an absolute HTTP(S) URL");
        }
        if let Some(source_hash) = &self.source_hash {
            if source_hash.len() != 64
                || !source_hash
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            {
                return Err("evidence packet source hash must be 64 hexadecimal characters");
            }
        }
        if !(0.0..=1.0).contains(&self.confidence) || self.confidence.is_nan() {
            return Err("evidence packet confidence must be between 0.0 and 1.0 inclusive");
        }
        if self.red_flags.len() > 16 {
            return Err("evidence packet red flags exceed the maximum count");
        }
        if self.missing.len() > 16 {
            return Err("evidence packet missing items exceed the maximum count");
        }
        for flag in &self.red_flags {
            if flag.trim().is_empty() || flag.len() > 500 {
                return Err("evidence packet red flag must be non-empty and at most 500 bytes");
            }
        }
        for item in &self.missing {
            if item.trim().is_empty() || item.len() > 500 {
                return Err("evidence packet missing item must be non-empty and at most 500 bytes");
            }
        }
        if self
            .record_url
            .as_ref()
            .is_some_and(|record_url| record_url.len() > 2_048)
        {
            return Err("evidence packet record URL exceeds its size limit");
        }
        if let Some(record_url) = &self.record_url {
            let parsed = url::Url::parse(record_url)
                .map_err(|_| "evidence packet record URL must be an absolute HTTP(S) URL")?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("evidence packet record URL must be an absolute HTTP(S) URL");
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

    fn sample_payload() -> EvidencePacketPayload {
        EvidencePacketPayload {
            schema_version: EVIDENCE_PACKET_SCHEMA_VERSION,
            packet_id: Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
            title: "Booking confirmation evidence".into(),
            source_url: "https://stomaton.example/evidence/625".into(),
            source_hash: Some("ab".repeat(32)),
            parse_summary: "PDF contains a confirmed sailing window.".into(),
            confidence: 0.91,
            red_flags: vec!["Consignee address is incomplete.".into()],
            missing: vec!["Bill of lading number".into()],
            recommendation: "Request the missing BL number before dispatch.".into(),
            record_url: Some("https://stomaton.example/cases/625".into()),
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
    fn confidence_must_be_in_unit_interval() {
        let mut high = sample_payload();
        high.confidence = 1.01;
        assert_eq!(
            high.validate(),
            Err("evidence packet confidence must be between 0.0 and 1.0 inclusive")
        );

        let mut low = sample_payload();
        low.confidence = -0.01;
        assert_eq!(
            low.validate(),
            Err("evidence packet confidence must be between 0.0 and 1.0 inclusive")
        );

        let mut nan = sample_payload();
        nan.confidence = f64::NAN;
        assert_eq!(
            nan.validate(),
            Err("evidence packet confidence must be between 0.0 and 1.0 inclusive")
        );

        let mut ok = sample_payload();
        ok.confidence = 0.0;
        assert!(ok.validate().is_ok());
        ok.confidence = 1.0;
        assert!(ok.validate().is_ok());
    }
}
