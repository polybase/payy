use crate::apps::model::{
    AppCatalogMetadata, AppCommand, AppCommandDocs, AppCommandExample, AppCommandParameter,
    AppManifest, AppPermissions, HostApi, RegistrySignature, WasmArtifact,
};

use super::render_app_command_help;

#[test]
fn command_help_renders_manifest_command_details() {
    let manifest = AppManifest {
        catalog: AppCatalogMetadata::default(),
        commands: vec![AppCommand {
            about: "Fetch a quote and prepare a swap".to_string(),
            docs: Some(AppCommandDocs {
                arguments: vec![AppCommandParameter {
                    default: None,
                    description: "Token to spend".to_string(),
                    kind: "string".to_string(),
                    name: "sell-token".to_string(),
                    required: true,
                    sensitive: false,
                    value_name: None,
                }],
                examples: vec![AppCommandExample {
                    command: "beam x uniswap swap USDC ETH 100".to_string(),
                    description: "Prepare a Base swap".to_string(),
                    title: "Swap".to_string(),
                }],
                invocation: "beam x uniswap swap <sell-token> <buy-token> <amount>".to_string(),
                options: Vec::new(),
                output_notes: vec!["Returns a pending approval".to_string()],
                summary: "Prepare a swap".to_string(),
            }),
            input_schema: serde_json::json!({}),
            name: "swap".to_string(),
            output_schema: serde_json::json!({}),
            sensitive_args: Vec::new(),
            usage: "swap <sell-token> <buy-token> <amount>".to_string(),
        }],
        description: "Uniswap app".to_string(),
        display_name: "Uniswap".to_string(),
        format_version: 1,
        host_api: HostApi::default(),
        icon: None,
        id: "uniswap".to_string(),
        min_beam_version: "0.1.2".to_string(),
        permissions: AppPermissions::default(),
        publisher: "Payy".to_string(),
        signature: RegistrySignature {
            algorithm: "sha256-dev".to_string(),
            key_id: "test".to_string(),
            value: "sha256:0".to_string(),
        },
        version: "1.0.0".to_string(),
        wasm: WasmArtifact {
            entrypoint: "beam_app_main".to_string(),
            sha256: "sha256:0".to_string(),
        },
    };

    let help = render_app_command_help(&manifest, &manifest.commands[0]);

    assert!(help.contains("Usage: beam x uniswap swap"));
    assert!(help.contains("Arguments:"));
    assert!(help.contains("sell-token"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("Returns a pending approval"));
}
