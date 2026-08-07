use buzz_core::kind::{KIND_STREAM_REVIEW_CARD, KIND_STREAM_TRIAGE_CARD};
use buzz_core::review_card::{ReviewCardPayload, ReviewDecision};
use buzz_core::triage_card::{TriageCardPayload, TriageGroup, TriageRisk};
use buzz_sdk::{build_review_card, build_triage_card, ThreadRef};
use nostr::{EventBuilder, EventId, Keys};
use uuid::Uuid;

fn sign(builder: EventBuilder) -> nostr::Event {
    builder
        .sign_with_keys(&Keys::generate())
        .expect("test event should sign")
}

fn tag_value<'a>(event: &'a nostr::Event, name: &str) -> Option<&'a str> {
    event.tags.iter().find_map(|tag| {
        let parts = tag.as_slice();
        (parts.first().map(String::as_str) == Some(name))
            .then(|| parts.get(1).map(String::as_str))
            .flatten()
    })
}

#[test]
fn triage_card_keeps_markdown_fallback_and_signed_structured_payload() {
    let channel_id = Uuid::new_v4();
    let payload = TriageCardPayload {
        schema_version: 1,
        triage_id: Uuid::new_v4(),
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
        ],
        proposed_action: "Open the oldest high-risk group first.".into(),
        record_url: Some("https://stomaton.example/triage/68".into()),
        shadow: true,
    };
    let fallback = "## Triage\n68 shadow drafts need triage\n\n**SHADOW — NOT DELIVERED**";

    let event = sign(
        build_triage_card(channel_id, &payload, fallback, None)
            .expect("valid triage card should build"),
    );

    assert_eq!(event.kind.as_u16(), KIND_STREAM_TRIAGE_CARD as u16);
    assert_eq!(event.content, fallback);
    assert_eq!(
        tag_value(&event, "h"),
        Some(channel_id.to_string().as_str())
    );
    assert_eq!(tag_value(&event, "shadow"), Some("1"));

    let encoded = tag_value(&event, "triage_card").expect("triage_card tag");
    let decoded: TriageCardPayload = serde_json::from_str(encoded).expect("valid payload JSON");
    assert_eq!(decoded, payload);

    let payload_hash = tag_value(&event, "payload_hash").expect("payload_hash tag");
    assert_eq!(payload_hash, payload.payload_hash().expect("payload hash"));
    assert_eq!(payload_hash.len(), 64);
}

#[test]
fn review_card_keeps_markdown_fallback_and_thread_refs() {
    let channel_id = Uuid::new_v4();
    let root_id = EventId::from_hex(&"11".repeat(32)).expect("root event id");
    let parent_id = EventId::from_hex(&"22".repeat(32)).expect("parent event id");
    let payload = ReviewCardPayload {
        schema_version: 1,
        review_id: Uuid::new_v4(),
        title: "Evidence card suite slice 3".into(),
        patch_ref: "https://github.com/block/buzz/pull/999".into(),
        evidence: vec!["Unit tests cover hash stability.".into()],
        test_summary: Some("cargo test -p buzz-core: ok".into()),
        decision: ReviewDecision::ChangesRequested,
        reviewer: Some("tyler".into()),
        record_url: None,
        shadow: true,
    };
    let fallback = "## Review\nChanges requested — SHADOW / NOT DELIVERED";
    let thread_ref = ThreadRef {
        root_event_id: root_id,
        parent_event_id: parent_id,
    };

    let event = sign(
        build_review_card(channel_id, &payload, fallback, Some(&thread_ref))
            .expect("valid review card should build"),
    );

    assert_eq!(event.kind.as_u16(), KIND_STREAM_REVIEW_CARD as u16);
    assert_eq!(event.content, fallback);
    assert_eq!(
        tag_value(&event, "h"),
        Some(channel_id.to_string().as_str())
    );
    assert_eq!(tag_value(&event, "shadow"), Some("1"));

    let encoded = tag_value(&event, "review_card").expect("review_card tag");
    let decoded: ReviewCardPayload = serde_json::from_str(encoded).expect("valid review JSON");
    assert_eq!(decoded, payload);

    let payload_hash = tag_value(&event, "payload_hash").expect("payload_hash tag");
    assert_eq!(payload_hash, payload.payload_hash().expect("payload hash"));

    let e_tags: Vec<Vec<String>> = event
        .tags
        .iter()
        .filter(|tag| tag.as_slice().first().map(String::as_str) == Some("e"))
        .map(|tag| tag.as_slice().to_vec())
        .collect();
    assert_eq!(
        e_tags[0],
        vec!["e".into(), root_id.to_hex(), "".into(), "root".into()]
    );
    assert_eq!(
        e_tags[1],
        vec!["e".into(), parent_id.to_hex(), "".into(), "reply".into()]
    );
}

#[test]
fn triage_card_rejects_empty_groups() {
    let payload = TriageCardPayload {
        schema_version: 1,
        triage_id: Uuid::new_v4(),
        title: "Triage".into(),
        groups: vec![],
        proposed_action: "Hold.".into(),
        record_url: None,
        shadow: true,
    };

    assert!(build_triage_card(Uuid::new_v4(), &payload, "fallback", None).is_err());
}

#[test]
fn review_card_rejects_non_http_record_url() {
    let payload = ReviewCardPayload {
        schema_version: 1,
        review_id: Uuid::new_v4(),
        title: "Review".into(),
        patch_ref: "abc123".into(),
        evidence: vec![],
        test_summary: None,
        decision: ReviewDecision::Pending,
        reviewer: None,
        record_url: Some("javascript:alert(1)".into()),
        shadow: true,
    };

    assert!(build_review_card(Uuid::new_v4(), &payload, "fallback", None).is_err());
}
