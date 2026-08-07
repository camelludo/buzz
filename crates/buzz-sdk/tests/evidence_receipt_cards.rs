use buzz_core::delivery_receipt::{DeliveryReceiptPayload, DeliveryReceiptStatus};
use buzz_core::evidence_packet::EvidencePacketPayload;
use buzz_core::kind::{KIND_STREAM_DELIVERY_RECEIPT, KIND_STREAM_EVIDENCE_PACKET};
use buzz_sdk::{build_delivery_receipt, build_evidence_packet, ThreadRef};
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
fn evidence_packet_keeps_markdown_fallback_and_signed_structured_payload() {
    let channel_id = Uuid::new_v4();
    let payload = EvidencePacketPayload {
        schema_version: 1,
        packet_id: Uuid::new_v4(),
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
    };
    let fallback =
        "## Evidence packet\nBooking confirmation evidence\n\n**SHADOW — NOT DELIVERED**";

    let event = sign(
        build_evidence_packet(channel_id, &payload, fallback, None)
            .expect("valid evidence packet should build"),
    );

    assert_eq!(event.kind.as_u16(), KIND_STREAM_EVIDENCE_PACKET as u16);
    assert_eq!(event.content, fallback);
    assert_eq!(
        tag_value(&event, "h"),
        Some(channel_id.to_string().as_str())
    );
    assert_eq!(tag_value(&event, "shadow"), Some("1"));

    let encoded = tag_value(&event, "evidence_packet").expect("evidence_packet tag");
    let decoded: EvidencePacketPayload = serde_json::from_str(encoded).expect("valid payload JSON");
    assert_eq!(decoded, payload);

    let payload_hash = tag_value(&event, "payload_hash").expect("payload_hash tag");
    assert_eq!(payload_hash, payload.payload_hash().expect("payload hash"));
    assert_eq!(payload_hash.len(), 64);
}

#[test]
fn delivery_receipt_keeps_markdown_fallback_and_thread_refs() {
    let channel_id = Uuid::new_v4();
    let root_id = EventId::from_hex(&"11".repeat(32)).expect("root event id");
    let parent_id = EventId::from_hex(&"22".repeat(32)).expect("parent event id");
    let payload = DeliveryReceiptPayload {
        schema_version: 1,
        receipt_id: Uuid::new_v4(),
        action_id: Uuid::new_v4(),
        status: DeliveryReceiptStatus::ProofMissing,
        detail: "Provider webhook did not include a message id.".into(),
        proof_url: None,
        shadow: true,
    };
    let fallback = "## Delivery receipt\nProof missing — SHADOW / NOT DELIVERED";
    let thread_ref = ThreadRef {
        root_event_id: root_id,
        parent_event_id: parent_id,
    };

    let event = sign(
        build_delivery_receipt(channel_id, &payload, fallback, Some(&thread_ref))
            .expect("valid delivery receipt should build"),
    );

    assert_eq!(event.kind.as_u16(), KIND_STREAM_DELIVERY_RECEIPT as u16);
    assert_eq!(event.content, fallback);
    assert_eq!(
        tag_value(&event, "h"),
        Some(channel_id.to_string().as_str())
    );
    assert_eq!(tag_value(&event, "shadow"), Some("1"));

    let encoded = tag_value(&event, "delivery_receipt").expect("delivery_receipt tag");
    let decoded: DeliveryReceiptPayload =
        serde_json::from_str(encoded).expect("valid receipt JSON");
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
fn evidence_packet_rejects_out_of_range_confidence() {
    let payload = EvidencePacketPayload {
        schema_version: 1,
        packet_id: Uuid::new_v4(),
        title: "Evidence".into(),
        source_url: "https://stomaton.example/evidence/1".into(),
        source_hash: None,
        parse_summary: "Summary.".into(),
        confidence: 1.5,
        red_flags: vec![],
        missing: vec![],
        recommendation: "Hold.".into(),
        record_url: None,
        shadow: true,
    };

    assert!(build_evidence_packet(Uuid::new_v4(), &payload, "fallback", None).is_err());
}

#[test]
fn delivery_receipt_rejects_non_http_proof_url() {
    let payload = DeliveryReceiptPayload {
        schema_version: 1,
        receipt_id: Uuid::new_v4(),
        action_id: Uuid::new_v4(),
        status: DeliveryReceiptStatus::Sent,
        detail: "Queued.".into(),
        proof_url: Some("javascript:alert(1)".into()),
        shadow: true,
    };

    assert!(build_delivery_receipt(Uuid::new_v4(), &payload, "fallback", None).is_err());
}
