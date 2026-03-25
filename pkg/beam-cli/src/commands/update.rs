use std::{
    ffi::{OsStr, OsString},
    io::Write,
    path::Path,
    process::{Command, ExitStatus},
};

use clap::Parser;
use contextful::ResultContextExt;
use serde_json::json;
use tempfile::NamedTempFile;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::{
    cli::Cli,
    display::ColorMode,
    error::Result,
    output::{CommandOutput, OutputMode, with_loading},
    runtime::InvocationOverrides,
    update_client::{
        UpdateInfo, available_update, current_version_string, download_update_bytes,
        verify_release_asset_bytes,
    },
};

pub async fn run_update(
    current_overrides: &InvocationOverrides,
    output_mode: OutputMode,
    color_mode: ColorMode,
) -> Result<()> {
    match with_loading(output_mode, "Checking for beam updates...", async {
        available_update().await
    })
    .await?
    {
        Some(update) => {
            apply_update(&update, output_mode).await?;
            CommandOutput::new(
                format!("Updated beam to {}", update.version),
                json!({
                    "tag_name": update.tag_name,
                    "updated": true,
                    "version": update.version.to_string(),
                }),
            )
            .compact(update.version.to_string())
            .print(output_mode)?;
            maybe_restart_after_update(current_overrides, output_mode, color_mode)
        }
        None => CommandOutput::new(
            format!("beam {} is already up to date", current_version_string()?),
            json!({ "updated": false }),
        )
        .print(output_mode),
    }
}

async fn apply_update(update: &UpdateInfo, output_mode: OutputMode) -> Result<()> {
    let bytes = with_loading(
        output_mode,
        format!("Downloading beam {}...", update.version),
        async { download_update_bytes(update).await },
    )
    .await?;
    verify_release_asset_bytes(&update.asset_name, &bytes, &update.asset_digest)?;
    let temp_file = NamedTempFile::new().context("create beam update temp file")?;
    std::fs::write(temp_file.path(), &bytes).context("write beam update temp file")?;

    #[cfg(unix)]
    {
        std::fs::set_permissions(temp_file.path(), std::fs::Permissions::from_mode(0o755))
            .context("set beam update permissions")?;
    }

    self_replace::self_replace(temp_file.path()).context("replace beam executable")?;
    Ok(())
}

fn maybe_restart_after_update(
    current_overrides: &InvocationOverrides,
    output_mode: OutputMode,
    color_mode: ColorMode,
) -> Result<()> {
    let Some(args) = restart_after_update_args(
        std::env::args_os(),
        current_overrides,
        output_mode,
        color_mode,
    )?
    else {
        return Ok(());
    };

    std::io::stdout()
        .flush()
        .context("flush beam update output")?;

    let executable = std::env::current_exe().context("resolve beam executable path")?;
    let status = restart_executable(&executable, args)?;
    let exit_code = if status.success() {
        0
    } else {
        status.code().unwrap_or(1)
    };

    std::process::exit(exit_code);
}

pub(crate) fn restart_after_update_args<I, S>(
    args: I,
    current_overrides: &InvocationOverrides,
    output_mode: OutputMode,
    color_mode: ColorMode,
) -> Result<Option<Vec<OsString>>>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let cli =
        Cli::try_parse_from(args.iter().cloned()).context("parse beam args for update restart")?;

    if !cli.is_interactive() {
        return Ok(None);
    }

    if cli.color == color_mode && cli.overrides() == *current_overrides && cli.output == output_mode
    {
        return Ok(Some(args.into_iter().skip(1).collect()));
    }

    Ok(Some(Vec::new()))
}

pub(crate) fn restart_executable<I, S>(executable: &Path, args: I) -> Result<ExitStatus>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(executable)
        .args(args)
        .status()
        .context("restart beam after update")
        .map_err(Into::into)
}
