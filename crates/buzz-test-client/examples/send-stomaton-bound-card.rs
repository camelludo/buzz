//! Send a DecisionCard with record_url binding for Stomaton poller (#1353).
//! Usage: cargo run -p buzz-test-client --example send-stomaton-bound-card -- \
//!   <hex-secret> <relay-ws-url> <channel-uuid> <actionId> <caseId lead:N>

use std::time::Duration;

use buzz_core::decision_card::{DecisionCardChoice, DecisionCardPayload};
use buzz_sdk::build_decision_card;
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
    let action_id = args.get(4).expect("actionId");
    let case_id = args.get(5).expect("caseId e.g. lead:302");
    let keys = Keys::parse(secret).expect("valid secret");

    let record_url = format!(
        "https://stomaton.sez.ai/dashboard/crm?actionId={}&caseId={}",
        action_id,
        case_id // URL encoding handled by URL if we use proper URL - keep simple
    );

    let payload = DecisionCardPayload {
        schema_version: 1,
        card_id: Uuid::new_v4(),
        title: format!("Shadow gate for action {}", action_id),
        situation: format!(
            "Stomaton ProposedAction #{} needs a human gate (case {}).",
            action_id, case_id
        ),
        recommendation: "Approve or reject in shadow mode — no external send.".into(),
        proposed_action: format!(
            "Record shadow decision for ProposedAction #{} ({})",
            action_id, case_id
        ),
        risk: "Shadow path cannot send externally.".into(),
        record_url: Some(record_url),
        choices: vec![DecisionCardChoice::Approve, DecisionCardChoice::Reject],
        expires_at: Some((chrono_now() + 86_400) as i64),
        shadow: true,
    };

    let md = format!(
        "## Shadow gate for action {}\n\nCase `{}` — **SHADOW / NOT DELIVERED**.\n\nApprove or reject; Stomaton re-checks.",
        action_id, case_id
    );

    let card = build_decision_card(channel, &payload, &md, None)
        .expect("card builder")
        .sign_with_keys(&keys)
        .expect("signed card");

    let mut client = BuzzTestClient::connect(relay, &keys)
        .await
        .expect("relay connection");
    let accepted = client.send_event(card.clone()).await.expect("publish card");
    assert!(
        accepted.accepted,
        "relay rejected card: {}",
        accepted.message
    );
    println!("card_id={} event_id={}", payload.card_id, card.id.to_hex());

    client
        .subscribe("card-check", vec![Filter::new().id(card.id)])
        .await
        .expect("query");
    let stored = client
        .collect_until_eose("card-check", Duration::from_secs(5))
        .await
        .expect("stored card query");
    assert_eq!(stored.len(), 1, "card not durably stored");
    println!("durable=true");
}

fn chrono_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
