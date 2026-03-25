use super::fixtures::test_app;
use crate::runtime::InvocationOverrides;

#[tokio::test]
async fn token_for_chain_uses_known_token_metadata_for_raw_address() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;

    let token = app
        .token_for_chain("0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", "base")
        .await
        .expect("resolve base usdc by address");

    assert_eq!(token.label, "USDC");
    assert_eq!(token.decimals, Some(6));
}

#[tokio::test]
async fn tracked_tokens_for_chain_falls_back_to_known_tokens_when_unconfigured() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    app.config_store
        .update(|config| {
            config.tracked_tokens.remove("ethereum");
        })
        .await
        .expect("remove tracked ethereum tokens");

    let tokens = app.tracked_tokens_for_chain("ethereum").await;
    let labels = tokens
        .into_iter()
        .map(|token| token.label)
        .collect::<Vec<_>>();

    assert_eq!(labels, vec!["USDC".to_string(), "USDT".to_string()]);
}
