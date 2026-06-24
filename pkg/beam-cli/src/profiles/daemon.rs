// lint-long-file-override allow-max-lines=220
use std::os::unix::fs::PermissionsExt;

use contextful::ResultContextExt;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    time::{Duration, timeout},
};
use web3::{Web3, transports::Http};

use crate::{
    cli::ProfileDaemonArgs,
    error,
    profiles::{
        Error, Result, ledger, policy,
        store::{self, integrity_key, now},
        wire::{DaemonRequest, DaemonResponse, WireSignedTransaction},
    },
    signer::{KeySigner, Signer},
};

pub async fn run(args: ProfileDaemonArgs) -> error::Result<()> {
    run_inner(args).await?;
    Ok(())
}

async fn run_inner(args: ProfileDaemonArgs) -> Result<()> {
    let mut stdin = String::new();
    tokio::io::stdin()
        .read_to_string(&mut stdin)
        .await
        .context("read profile daemon key")?;
    let secret_key = hex::decode(stdin.trim()).map_err(|_| Error::InvalidDaemonRequest)?;
    let signer = KeySigner::from_slice(&secret_key).map_err(|_| Error::InvalidDaemonRequest)?;
    let integrity_key = integrity_key(&secret_key);

    if let Some(parent) = args.socket.parent() {
        fs::create_dir_all(parent)
            .await
            .context("create profile daemon socket directory")?;
    }
    if args.socket.exists() {
        fs::remove_file(&args.socket)
            .await
            .context("remove stale profile daemon socket")?;
    }
    let listener = UnixListener::bind(&args.socket).context("bind profile daemon socket")?;
    std::fs::set_permissions(&args.socket, std::fs::Permissions::from_mode(0o600))
        .context("set profile daemon socket permissions")?;

    loop {
        if now() >= args.expires_at {
            break;
        }
        let remaining = args.expires_at.saturating_sub(now()).max(1);
        let accepted = timeout(Duration::from_secs(remaining), listener.accept()).await;
        let Ok(Ok((stream, _addr))) = accepted else {
            break;
        };
        let response = handle_request(&args, &signer, &integrity_key, stream).await;
        if matches!(response, DaemonLoopResponse::Lock) {
            break;
        }
    }

    if args.socket.exists() {
        let _ = fs::remove_file(&args.socket).await;
    }
    Ok(())
}

enum DaemonLoopResponse {
    Continue,
    Lock,
}

async fn handle_request(
    args: &ProfileDaemonArgs,
    signer: &KeySigner,
    integrity_key: &[u8],
    mut stream: UnixStream,
) -> DaemonLoopResponse {
    let response = match read_request(&mut stream).await {
        Ok(request) => execute_request(args, signer, integrity_key, request).await,
        Err(err) => Err(err),
    };
    let lock = matches!(response, Ok(DaemonResponse::Ok)) && now() >= args.expires_at;
    let response = response.unwrap_or_else(|err| DaemonResponse::Error {
        message: if matches!(err, Error::SessionTokenRejected) {
            "session-token-rejected".to_string()
        } else {
            err.to_string()
        },
    });
    let _ = write_response(&mut stream, &response).await;
    if lock || matches!(response, DaemonResponse::Ok) && should_stop(args).await {
        DaemonLoopResponse::Lock
    } else {
        DaemonLoopResponse::Continue
    }
}

async fn execute_request(
    args: &ProfileDaemonArgs,
    signer: &KeySigner,
    integrity_key: &[u8],
    request: DaemonRequest,
) -> Result<DaemonResponse> {
    ensure_session(args, request_token(&request)?)?;
    match request {
        DaemonRequest::Ping { .. } => Ok(DaemonResponse::Ok),
        DaemonRequest::Lock { .. } => {
            fs::write(args.root.join("profiles").join("lock-sentinel"), b"")
                .await
                .context("write profile daemon lock sentinel")?;
            Ok(DaemonResponse::Ok)
        }
        DaemonRequest::Rollback { ledger_id, .. } => {
            ledger::mark_rolled_back(&args.root, integrity_key, &ledger_id).await?;
            Ok(DaemonResponse::Ok)
        }
        DaemonRequest::Sign {
            intent,
            transaction,
            ..
        } => {
            let profile = store::find(&args.root, &args.profile).await?;
            store::verify_profile(&profile, integrity_key)?;
            let ledger_store = ledger::load(&args.root).await?;
            let ledger_state = ledger::verified_state(&ledger_store, integrity_key).await?;
            let decision = policy::authorize(&profile, &ledger_state, &intent, &transaction)?;
            let tx = transaction.to_parameters()?;
            let dummy = dummy_web3()?;
            let signed = signer
                .sign_transaction(&dummy, tx)
                .await
                .map_err(|_| Error::InvalidDaemonRequest)?;
            let entry = ledger::append_signed(
                &args.root,
                integrity_key,
                &profile,
                &decision.grant_id,
                &intent,
                transaction.gas.clone(),
                format!("{:#x}", signed.transaction_hash),
            )
            .await?;
            Ok(DaemonResponse::Signed {
                ledger_id: entry.id,
                transaction: WireSignedTransaction::from_signed(&signed),
            })
        }
    }
}

fn request_token(request: &DaemonRequest) -> Result<&str> {
    match request {
        DaemonRequest::Ping { token }
        | DaemonRequest::Lock { token }
        | DaemonRequest::Rollback { token, .. }
        | DaemonRequest::Sign { token, .. } => Ok(token),
    }
}

fn ensure_session(args: &ProfileDaemonArgs, token: &str) -> Result<()> {
    if args.expires_at <= now() {
        return Err(Error::SessionExpired {
            profile: args.profile.clone(),
        });
    }
    if token == args.session {
        Ok(())
    } else {
        Err(Error::SessionTokenRejected)
    }
}

async fn read_request(stream: &mut UnixStream) -> Result<DaemonRequest> {
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .await
        .context("read profile daemon request")?;
    serde_json::from_slice(&bytes)
        .context("decode profile daemon request")
        .map_err(Into::into)
}

async fn write_response(stream: &mut UnixStream, response: &DaemonResponse) -> Result<()> {
    let bytes = serde_json::to_vec(response).context("encode profile daemon response")?;
    stream
        .write_all(&bytes)
        .await
        .context("write profile daemon response")?;
    Ok(())
}

async fn should_stop(args: &ProfileDaemonArgs) -> bool {
    let sentinel = args.root.join("profiles").join("lock-sentinel");
    if sentinel.exists() {
        let _ = fs::remove_file(sentinel).await;
        true
    } else {
        false
    }
}

fn dummy_web3() -> Result<Web3<Http>> {
    let transport = Http::new("http://127.0.0.1:1").context("create daemon signing transport")?;
    Ok(Web3::new(transport))
}
