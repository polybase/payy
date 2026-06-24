use contextful::ResultContextExt;
use serde_json::json;

use crate::{
    error::Result, human_output::sanitize_control_chars, output::CommandOutput, runtime::BeamApp,
};

pub(crate) async fn use_wallet(app: &BeamApp, name: &str) -> Result<()> {
    let wallet = app.resolve_wallet(name).await?;
    let name = wallet.name.clone();

    app.config_store
        .update(|config| config.default_wallet = Some(name.clone()))
        .await
        .context("persist beam default wallet")?;

    let name = sanitize_control_chars(&name);
    CommandOutput::new(
        format!("Default wallet set to {name} ({})", wallet.address),
        json!({
            "address": wallet.address,
            "name": wallet.name,
        }),
    )
    .compact(name)
    .print(app.output_mode)
}
