use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use clap::Args;
use contextful::{ErrorContextExt, ResultContextExt};
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_LENGTH, ETAG, LAST_MODIFIED};
use serde::{Deserialize, Serialize};

use crate::error::{Result, XTaskError, workspace_root};

const REVI_LINUX_URL: &str = "https://storage.googleapis.com/payy-public-fixtures/1875db7e1fc13e88f31c4fc4/revi/latest/x86_64-unknown-linux-musl/revi";
const REVI_MAC_URL: &str = "https://storage.googleapis.com/payy-public-fixtures/1875db7e1fc13e88f31c4fc4/revi/latest/aarch64-apple-darwin/revi";

#[derive(Args)]
#[command(trailing_var_arg = true, allow_hyphen_values = true)]
pub struct ReviArgs {
    #[arg(value_name = "REVI_ARGS")]
    pub args: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HeadInfo {
    etag: Option<String>,
    last_modified: Option<String>,
    content_length: Option<u64>,
}

impl HeadInfo {
    fn is_useful(&self) -> bool {
        self.etag.is_some() || self.last_modified.is_some() || self.content_length.is_some()
    }
}

pub fn run_revi(args: ReviArgs) -> Result<()> {
    let url = revi_url()?;
    let repo_root = workspace_root()?;
    let home_dir = home::home_dir().ok_or(XTaskError::HomeDir)?;
    let cache_dir = home_dir.join(".polybase").join("revi");
    fs::create_dir_all(&cache_dir).context("create revi cache directory")?;

    let binary_path = cache_dir.join("revi");
    let head_path = cache_dir.join("revi.head.json");
    let head_info = fetch_head(url)?;
    let stored_head = read_head(&head_path)?;
    let should_download =
        !binary_path.exists() || !head_info.is_useful() || stored_head.as_ref() != Some(&head_info);

    if should_download {
        download_revi(&cache_dir, &binary_path, url)?;
        write_head(&head_path, &head_info)?;
    }

    run_revi_command(&repo_root, &binary_path, &args.args)
}

fn revi_url() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok(REVI_LINUX_URL),
        ("macos", "aarch64") => Ok(REVI_MAC_URL),
        (os, arch) => Err(XTaskError::UnsupportedPlatform { os, arch }),
    }
}

fn fetch_head(url: &str) -> Result<HeadInfo> {
    let client = Client::new();
    let response = client
        .head(url)
        .send()
        .context("request revi head")?
        .error_for_status()
        .context("request revi head")?;

    Ok(HeadInfo {
        etag: response
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        last_modified: response
            .headers()
            .get(LAST_MODIFIED)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        content_length: response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok()),
    })
}

fn read_head(path: &Path) -> Result<Option<HeadInfo>> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(XTaskError::Io(err.wrap_err("read revi head info"))),
    };

    Ok(Some(
        serde_json::from_str(&contents).context("parse revi head info")?,
    ))
}

fn write_head(path: &Path, head_info: &HeadInfo) -> Result<()> {
    if !head_info.is_useful() {
        return Ok(());
    }

    let mut payload = serde_json::to_string(head_info).context("serialize revi head info")?;
    payload.push('\n');
    fs::write(path, payload).context("write revi head info")?;
    Ok(())
}

fn download_revi(cache_dir: &Path, binary_path: &Path, url: &str) -> Result<()> {
    let client = Client::new();
    let mut response = client
        .get(url)
        .send()
        .context("download revi")?
        .error_for_status()
        .context("download revi")?;
    let mut temp_file =
        tempfile::NamedTempFile::new_in(cache_dir).context("create revi temp file")?;

    io::copy(&mut response, temp_file.as_file_mut()).context("write revi binary")?;
    temp_file
        .as_file_mut()
        .sync_all()
        .context("sync revi binary")?;

    #[cfg(unix)]
    {
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(temp_file.path(), permissions).context("set revi permissions")?;
    }

    if binary_path.exists() {
        fs::remove_file(binary_path).context("remove existing revi binary")?;
    }

    match temp_file.persist(binary_path) {
        Ok(_) => Ok(()),
        Err(err) => Err(XTaskError::Io(err.error.wrap_err("persist revi binary"))),
    }
}

fn run_revi_command(repo_root: &Path, binary_path: &Path, args: &[OsString]) -> Result<()> {
    let status = Command::new(binary_path)
        .args(args)
        .current_dir(repo_root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("run revi")?;

    if !status.success() {
        return Err(XTaskError::CommandFailure {
            program: "revi",
            status: status.code(),
            stderr: String::new(),
        });
    }

    Ok(())
}
