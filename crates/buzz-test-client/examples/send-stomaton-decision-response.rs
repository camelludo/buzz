//! Publish kind 40010 DecisionResponse for a card.
//! Usage: cargo run -p buzz-test-client --example send-stomaton-decision-response -- \
//!   <hex-secret> <relay-ws> <channel-uuid> <card-event-id> <card-uuid> <payload-hash> <approve|reject>

use std::time::Duration;

use buzz_core::decision_card::{DecisionCardChoice, DecisionResponsePayload};
use buzz_sdk::{build_decision_response, ThreadRef};
use buzz_test_client::BuzzTestClient;
use nostr::{EventId, Filter, Keys};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("rustls");
    let args: Vec<String> = std::env::args().collect();
    let secret = &args[1];
    let relay = &args[2];
    let channel = Uuid::parse_str(&args[3]).expect("channel");
    let card_event = EventId::from_hex(&args[4]).expect("card event id");
    let card_id = Uuid::parse_str(&args[5]).expect("card uuid");
    let payload_hash = args[6].clone();
    let decision = match args[7].as_str() {
        "approve" => DecisionCardChoice::Approve,
        "reject" => DecisionCardChoice::Reject,
        other => panic!("decision must be approve|reject, got {other}"),
    };
    let keys = Keys::parse(secret).expect("keys");

    let payload = DecisionResponsePayload {
        schema_version: 1,
        action_id: Uuid::new_v4(),
        card_id,
        decision,
        payload_hash,
        note: Some("automated shadow proof (#1353)".into()),
        shadow: true,
    };

    let thread = ThreadRef {
        root_event_id: card_event,
        parent_event_id: card_event,
    };
    let md = match decision {
        DecisionCardChoice::Approve => "✅ Approved — SHADOW / NOT DELIVERED",
        DecisionCardChoice::Reject => "⛔ Rejected — SHADOW / NOT DELIVERED",
        _ => "Decision — SHADOW / NOT DELIVERED",
    };

    let event = build_decision_response(channel, &payload, md, &thread)
        .expect("build")
        .sign_with_keys(&keys)
        .expect("sign");

    let mut client = BuzzTestClient::connect(relay, &keys)
        .await
        .expect("connect");
    let accepted = client.send_event(event.clone()).await.expect("publish");
    assert!(accepted.accepted, "rejected: {}", accepted.message);
    println!("response_event_id={}", event.id.to_hex());

    client
        .subscribe("resp-check", vec![Filter::new().id(event.id)])
        .await
        .expect("sub");
    let stored = client
        .collect_until_eose("resp-check", Duration::from_secs(5))
        .await
        .expect("eose");
    assert_eq!(stored.len(), 1);
    println!("durable=true");
}
