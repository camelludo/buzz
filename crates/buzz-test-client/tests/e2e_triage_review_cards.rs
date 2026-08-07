//! Non-production relay canary for triage and review cards.

use std::time::Duration;

use buzz_core::review_card::{ReviewCardPayload, ReviewDecision};
use buzz_core::triage_card::{TriageCardPayload, TriageGroup, TriageRisk};
use buzz_sdk::{build_review_card, build_triage_card};
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
async fn signed_shadow_triage_and_review_round_trip_durably() {
    let keys = Keys::parse(TYLER_TEST_SECRET).expect("fixture key");
    let channel_id = Uuid::parse_str(GENERAL_CHANNEL_ID).expect("fixture channel");

    let triage_payload = TriageCardPayload {
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
    };
    let triage = build_triage_card(
        channel_id,
        &triage_payload,
        "## Triage\n68 shadow drafts need triage\n\n**SHADOW — NOT DELIVERED**",
        None,
    )
    .expect("triage builder")
    .sign_with_keys(&keys)
    .expect("signed triage");

    let mut client = BuzzTestClient::connect(&relay_url(), &keys)
        .await
        .expect("authenticated relay connection");
    let accepted = client
        .send_event(triage.clone())
        .await
        .expect("publish triage");
    assert!(
        accepted.accepted,
        "relay rejected triage card: {}",
        accepted.message
    );

    client
        .subscribe("triage-card", vec![Filter::new().id(triage.id)])
        .await
        .expect("query triage");
    let stored_triage = client
        .collect_until_eose("triage-card", Duration::from_secs(5))
        .await
        .expect("stored triage query");
    assert_eq!(stored_triage, vec![triage.clone()]);

    let review_payload = ReviewCardPayload {
        schema_version: 1,
        review_id: Uuid::new_v4(),
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
    };
    let review = build_review_card(
        channel_id,
        &review_payload,
        "## Review\nPending — SHADOW / NOT DELIVERED",
        None,
    )
    .expect("review builder")
    .sign_with_keys(&keys)
    .expect("signed review");
    let accepted = client
        .send_event(review.clone())
        .await
        .expect("publish review");
    assert!(
        accepted.accepted,
        "relay rejected review card: {}",
        accepted.message
    );

    client
        .subscribe("review-card", vec![Filter::new().id(review.id)])
        .await
        .expect("query review");
    let stored_reviews = client
        .collect_until_eose("review-card", Duration::from_secs(5))
        .await
        .expect("stored review query");
    assert_eq!(stored_reviews, vec![review.clone()]);
    assert!(stored_reviews[0]
        .tags
        .iter()
        .any(|tag| tag.as_slice().first().map(String::as_str) == Some("review_card")));
    assert!(stored_reviews[0]
        .tags
        .iter()
        .any(|tag| tag.as_slice() == ["shadow", "1"]));
    assert!(stored_triage[0]
        .tags
        .iter()
        .any(|tag| tag.as_slice().first().map(String::as_str) == Some("triage_card")));

    client.disconnect().await.expect("clean disconnect");
}
