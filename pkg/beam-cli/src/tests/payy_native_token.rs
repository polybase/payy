use super::fixtures::test_app_with_output;
use crate::{
    cli::{TokenAction, TokenAddArgs},
    commands::tokens,
    known_tokens::{
        PAYY_NATIVE_ERC20_ADDRESS, PAYY_NATIVE_TOKEN_DECIMALS, PAYY_NATIVE_TOKEN_LABEL,
    },
    output::OutputMode,
    runtime::InvocationOverrides,
};

const CUSTOM_NATIVE_ERC20_ADDRESS: &str = "0x0000000000000000000000000000000000000bee";

#[tokio::test]
async fn payy_native_erc20_is_available_from_builtin_defaults() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    app.config_store
        .update(|config| {
            config.known_tokens.remove("payy-testnet");
            config.tracked_tokens.remove("payy-testnet");
        })
        .await
        .expect("remove persisted payy token config");

    let token = app
        .token_for_chain(PAYY_NATIVE_TOKEN_LABEL, "payy-testnet")
        .await
        .expect("resolve payy native erc20");
    let tracked = app.tracked_tokens_for_chain("payy-testnet").await;

    assert_eq!(format!("{:#x}", token.address), PAYY_NATIVE_ERC20_ADDRESS);
    assert_eq!(token.decimals, Some(PAYY_NATIVE_TOKEN_DECIMALS));
    assert_eq!(token.label, PAYY_NATIVE_TOKEN_LABEL);
    assert_eq!(tracked[0].label, PAYY_NATIVE_TOKEN_LABEL);
}

#[tokio::test]
async fn tokens_add_allows_native_label_for_explicit_erc20() {
    let (_temp_dir, app) = test_app_with_output(
        OutputMode::Quiet,
        InvocationOverrides {
            chain: Some("base".to_string()),
            ..InvocationOverrides::default()
        },
    )
    .await;

    tokens::run(
        &app,
        Some(TokenAction::Add(TokenAddArgs {
            token: Some(CUSTOM_NATIVE_ERC20_ADDRESS.to_string()),
            label: Some(PAYY_NATIVE_TOKEN_LABEL.to_string()),
            decimals: Some(PAYY_NATIVE_TOKEN_DECIMALS),
        })),
    )
    .await
    .expect("add native-labeled erc20");

    let config = app.config_store.get().await;
    let (_, token) = config
        .known_token_by_label("base", PAYY_NATIVE_TOKEN_LABEL)
        .expect("persist native-labeled erc20");
    assert_eq!(token.address, CUSTOM_NATIVE_ERC20_ADDRESS);
    assert_eq!(token.decimals, PAYY_NATIVE_TOKEN_DECIMALS);
    drop(config);

    tokens::run(
        &app,
        Some(TokenAction::Remove {
            token: PAYY_NATIVE_TOKEN_LABEL.to_string(),
        }),
    )
    .await
    .expect("remove native-labeled erc20");

    let config = app.config_store.get().await;
    assert!(
        !config
            .tracked_token_keys_for_chain("base")
            .iter()
            .any(|label| label == "NATIVE")
    );
}
