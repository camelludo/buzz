//! Non-production relay canary for evidence packet and delivery receipt cards.

use std::time::Duration;

use buzz_core::delivery_receipt::{DeliveryReceiptPayload, DeliveryReceiptStatus};
use buzz_core::evidence_packet::EvidencePacketPayload;
use buzz_sdk::{build_delivery_receipt, build_evidence_packet};
use buzz_test_client::BuzzTestClient;
use nostr::{Filter, Keys};
use uuid::Uuid;

const TYLER_TEST_SECRET: &str = "3dbaebadb5dfd777ff25149ee230d907a15a9e1294b40b830661e65bb42f6c03";
const GENERAL_CHANNEL_ID: &str = "9f28288a-d724-587a-9709-92dc7f967110";

fn relay_url() -> String {
    std::env::var("RELAY_URL").unwrap_or_else(|_| "ws://localhost:3000".to_string())
}

#[tokio::test]
#[ignore = "requires a seeded non-production relay"]
async fn signed_shadow_evidence_and_receipt_round_trip_durably() {
    let keys = Keys::parse(TYLER_TEST_SECRET).expect("fixture key");
    let channel_id = Uuid::parse_str(GENERAL_CHANNEL_ID).expect("fixture channel");

    let evidence_payload = EvidencePacketPayload {
        schema_version: 1,
        packet_id: Uuid::new_v4(),
        title: "Booking confirmation evidence".into(),
        source_url: "https://stomaton.example/evidence/625".into(),
        source_hash: Some("ab".repeat(32)),
        parse_summary: "Historical case #625 has a confirmed sailing window.".into(),
        confidence: 0.91,
        red_flags: vec!["Consignee address is incomplete.".into()],
        missing: vec!["Bill of lading number".into()],
        recommendation: "Request the missing BL number before dispatch.".into(),
        record_url: Some("https://stomaton.example/cases/625".into()),
        shadow: true,
    };
    let evidence = build_evidence_packet(
        channel_id,
        &evidence_payload,
        "## Evidence packet\nBooking confirmation evidence\n\n**SHADOW — NOT DELIVERED**",
        None,
    )
    .expect("evidence builder")
    .sign_with_keys(&keys)
    .expect("signed evidence");

    let mut client = BuzzTestClient::connect(&relay_url(), &keys)
        .await
        .expect("authenticated relay connection");
    let accepted = client
        .send_event(evidence.clone())
        .await
        .expect("publish evidence");
    assert!(
        accepted.accepted,
        "relay rejected evidence packet: {}",
        accepted.message
    );

    client
        .subscribe("evidence-packet", vec![Filter::new().id(evidence.id)])
        .await
        .expect("query evidence");
    let stored_evidence = client
        .collect_until_eose("evidence-packet", Duration::from_secs(5))
        .await
        .expect("stored evidence query");
    assert_eq!(stored_evidence, vec![evidence.clone()]);

    let receipt_payload = DeliveryReceiptPayload {
        schema_version: 1,
        receipt_id: Uuid::new_v4(),
        action_id: Uuid::new_v4(),
        status: DeliveryReceiptStatus::Delivered,
        detail: "Non-production shadow delivery confirmed.".into(),
        proof_url: Some("https://stomaton.example/proofs/625".into()),
        shadow: true,
    };
    let receipt = build_delivery_receipt(
        channel_id,
        &receipt_payload,
        "## Delivery receipt\nDelivered — SHADOW / NOT DELIVERED",
        None,
    )
    .expect("receipt builder")
    .sign_with_keys(&keys)
    .expect("signed receipt");
    let accepted = client
        .send_event(receipt.clone())
        .await
        .expect("publish receipt");
    assert!(
        accepted.accepted,
        "relay rejected delivery receipt: {}",
        accepted.message
    );

    client
        .subscribe("delivery-receipt", vec![Filter::new().id(receipt.id)])
        .await
        .expect("query receipt");
    let stored_receipts = client
        .collect_until_eose("delivery-receipt", Duration::from_secs(5))
        .await
        .expect("stored receipt query");
    assert_eq!(stored_receipts, vec![receipt.clone()]);
    assert!(stored_receipts[0]
        .tags
        .iter()
        .any(|tag| tag.as_slice().first().map(String::as_str) == Some("delivery_receipt")));
    assert!(stored_receipts[0]
        .tags
        .iter()
        .any(|tag| tag.as_slice() == ["shadow", "1"]));
    assert!(stored_evidence[0]
        .tags
        .iter()
        .any(|tag| tag.as_slice().first().map(String::as_str) == Some("evidence_packet")));

    client.disconnect().await.expect("clean disconnect");
}
