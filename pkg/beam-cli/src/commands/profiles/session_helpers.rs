#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    io::Write,
    process::{Command as ProcessCommand, Stdio},
    time::Duration,
};

use contextful::ResultContextExt;
use tokio::time::sleep;

use crate::{
    error::Result,
    profiles::{Error as ProfileError, session::ProfileSession, signer},
    runtime::BeamApp,
};

pub fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn spawn_daemon(
    app: &BeamApp,
    profile: &str,
    socket: &std::path::Path,
    token: &str,
    expires_at: u64,
    secret_key: &[u8],
) -> Result<()> {
    let executable = std::env::current_exe().context("resolve beam executable")?;
    let mut command = ProcessCommand::new(executable);
    command
        .arg("--no-update-check")
        .arg("__profile-daemon")
        .arg("--root")
        .arg(&app.paths.root)
        .arg("--profile")
        .arg(profile)
        .arg("--socket")
        .arg(socket)
        .arg("--session")
        .arg(token)
        .arg("--expires-at")
        .arg(expires_at.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        // SAFETY: `pre_exec` runs in the child after fork and before exec. The
        // closure only installs an ignored `SIGHUP` disposition and calls
        // `setsid(2)`, both async-signal-safe libc operations, so the profile
        // daemon survives short-lived agent PTYs. No memory shared with other
        // threads is accessed.
        unsafe {
            command.pre_exec(|| {
                libc::signal(libc::SIGHUP, libc::SIG_IGN);
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    let mut child = command.spawn().context("spawn beam profile daemon")?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or(ProfileError::InvalidDaemonRequest)?;
    stdin
        .write_all(hex::encode(secret_key).as_bytes())
        .context("send profile daemon key")?;
    Ok(())
}

pub async fn wait_for_daemon(session: &ProfileSession) -> Result<()> {
    for _ in 0..20 {
        if signer::ping(session).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(50)).await;
    }
    Err(ProfileError::DaemonConnectionFailed {
        profile: session.profile.clone(),
    }
    .into())
}
