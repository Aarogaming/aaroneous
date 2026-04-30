#[tokio::test]
async fn test_token_bucket_throttling() {
    use std::time::Duration;
    use aaroneous::SystemBiology;

    let mut biology = SystemBiology::new();

    // Test: Token regeneration at default expression rate
    biology.update_metabolism();
    assert_eq!(biology.tokens, 10.0, "Tokens should max out at 10 after update.");

    // Test: Token consumption
    assert!(biology.consume_catalyst(), "Catalyst consumption should succeed when tokens are >= 1.");

    // Exhaust tokens
    for _ in 0..9 {
        biology.consume_catalyst();
    }
    assert!(!biology.consume_catalyst(), "Catalyst consumption should fail when tokens are depleted.");

    // Test: Regeneration after depletion
    tokio::time::sleep(Duration::from_secs(10)).await;
    biology.update_metabolism();
    assert!(biology.tokens > 0.0, "Tokens should regenerate after some time.");

    // Test: Zero expression rate
    biology.expression_rate = 0.0;
    let prev_tokens = biology.tokens;
    tokio::time::sleep(Duration::from_secs(5)).await;
    biology.update_metabolism();
    assert_eq!(biology.tokens, prev_tokens, "Tokens should not regenerate when expression_rate is 0.");
}
