//! Acceptance proof: publish one evidence packet (kind 40011) and one
//! delivery receipt (kind 40012) to a Buzz channel, verify both are durable.
//! Usage: send-acceptance-cards <hex-secret> <relay-ws-url> <channel-uuid>

use std::time::Duration;

use buzz_core::delivery_receipt::{DeliveryReceiptPayload, DeliveryReceiptStatus};
use buzz_core::evidence_packet::EvidencePacketPayload;
use buzz_sdk::{build_delivery_receipt, build_evidence_packet};
use buzz_test_client::BuzzTestClient;
use nostr::{Filter, Keys};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls provider");
    let args: Vec<String> = std::env::args().collect();
    let secret = args.get(1).expect("secret");
    let relay = args.get(2).expect("relay url");
    let channel = Uuid::parse_str(args.get(3).expect("channel")).expect("channel uuid");
    let keys = Keys::parse(secret).expect("valid secret");

    let packet = EvidencePacketPayload {
        schema_version: 1,
        packet_id: Uuid::new_v4(),
        title: "Acceptance proof: 5-card suite live on g2s".into(),
        source_url: "https://github.com/camelludo/buzz/commit/399ee76bd36d00642c6f1fe90272207052ad0613".into(),
        source_hash: None,
        parse_summary: "Sealed image 399ee76bd deployed to buzz.sez.ai by the CI watcher; relay healthy; kinds 40009-40014 accepted. This packet is the live-write proof for kind 40011.".into(),
        confidence: 0.97,
        red_flags: vec![],
        missing: vec!["Kubilay has not joined yet; welcome card still armed".into()],
        recommendation: "No action. Suite is live; treat this packet as the acceptance record.".into(),
        record_url: None,
        shadow: true,
    };
    let packet_event = build_evidence_packet(
        channel,
        &packet,
        "## Acceptance proof (lab)\n\nEvidence packet kind **40011** written live against `buzz.sez.ai` running `399ee76bd`.",
        None,
    )
    .expect("packet builder")
    .sign_with_keys(&keys)
    .expect("signed packet");

    let receipt = DeliveryReceiptPayload {
        schema_version: 1,
        receipt_id: Uuid::new_v4(),
        action_id: packet.packet_id,
        status: DeliveryReceiptStatus::Delivered,
        detail: "Auto-deploy of sealed image 399ee76bd to the g2s relay completed; health check green; this receipt is the live-write proof for kind 40012.".into(),
        proof_url: Some("https://buzz.sez.ai".into()),
        shadow: true,
    };
    let receipt_event = build_delivery_receipt(
        channel,
        &receipt,
        "## Delivery receipt (lab)\n\nReceipt kind **40012** closing the redeploy action, written live against `buzz.sez.ai`.",
        None,
    )
    .expect("receipt builder")
    .sign_with_keys(&keys)
    .expect("signed receipt");

    let mut client = BuzzTestClient::connect(relay, &keys)
        .await
        .expect("relay connection");

    for (label, event) in [("evidence_packet", &packet_event), ("delivery_receipt", &receipt_event)] {
        let accepted = client.send_event(event.clone()).await.expect("publish");
        assert!(accepted.accepted, "relay rejected {label}: {}", accepted.message);
        println!("{label} event_id={}", event.id.to_hex());
    }

    client
        .subscribe(
            "acceptance-check",
            vec![Filter::new().ids([packet_event.id, receipt_event.id])],
        )
        .await
        .expect("query");
    let stored = client
        .collect_until_eose("acceptance-check", Duration::from_secs(5))
        .await
        .expect("stored query");
    assert_eq!(stored.len(), 2, "cards not durably stored");
    println!("durable=true");
}
