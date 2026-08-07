//! Send a native decision card (kind 40009) to a Buzz channel.
//! Usage: cargo run -p buzz-test-client --example send-card -- <hex-secret> <relay-ws-url> <channel-uuid>

use std::time::Duration;

use buzz_core::decision_card::{DecisionCardChoice, DecisionCardPayload};
use buzz_sdk::build_decision_card;
use buzz_test_client::BuzzTestClient;
use nostr::{Filter, Keys};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let secret = args.get(1).expect("secret");
    let relay = args.get(2).expect("relay url");
    let channel = Uuid::parse_str(args.get(3).expect("channel")).expect("channel uuid");
    let keys = Keys::parse(secret).expect("valid secret");

    let payload = DecisionCardPayload {
        schema_version: 1,
        card_id: Uuid::new_v4(),
        title: "Welcome to the new g2s home".into(),
        situation: "Buzz moved to our own relay (buzz.sez.ai): 5 lanes with operating contracts, you are Admin, decision cards now render natively.".into(),
        recommendation: "Skim each lane's Canvas, then acknowledge this card.".into(),
        proposed_action: "No external action. Acknowledge = you are oriented; Escalate = you want a walkthrough.".into(),
        risk: "None. Old hosted community stays as read-only archive.".into(),
        record_url: Some("https://github.com/Go2Stone/freightman/issues/1258".into()),
        choices: vec![
            DecisionCardChoice::Approve,
            DecisionCardChoice::Escalate,
            DecisionCardChoice::Reject,
        ],
        expires_at: Some(2_100_000_000),
        shadow: false,
    };

    let card = build_decision_card(
        channel,
        &payload,
        "## Welcome to the new g2s home\n\nBuzz moved to **our own relay** (`buzz.sez.ai`).\n\n- 5 lanes with Canvas contracts: approvals · notifications · alerts · reports · general\n- You are **Admin**\n- Decision cards like this one now render natively\n- Aux agents Bumble/Honey/Fizz proved on this relay\n\n**Skim each lane's Canvas, then acknowledge.** Escalate if you want a walkthrough.",
        None,
    )
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
