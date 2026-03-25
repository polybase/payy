// lint-long-file-override allow-max-lines=300
use std::{
    future::Future,
    io::{IsTerminal, Write},
    sync::mpsc::{self, RecvTimeoutError},
    thread::{self, JoinHandle},
    time::Duration,
};

use clap::ValueEnum;
use contextful::ResultContextExt;
use serde_json::{Value, json};

use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum OutputMode {
    #[default]
    Default,
    Json,
    Yaml,
    Markdown,
    Compact,
    Quiet,
}

const LOADER_FRAMES: [&str; 4] = ["-", "\\", "|", "/"];
const LOADER_TICK: Duration = Duration::from_millis(120);

#[derive(Clone)]
pub struct LoadingHandle {
    control_tx: Option<mpsc::Sender<LoadingControl>>,
}

#[derive(Clone, Debug)]
pub struct CommandOutput {
    pub(crate) compact: Option<String>,
    pub(crate) default: String,
    markdown: Option<String>,
    pub(crate) value: Value,
}

impl CommandOutput {
    pub fn new(default: impl Into<String>, value: Value) -> Self {
        Self {
            compact: None,
            default: default.into(),
            markdown: None,
            value,
        }
    }

    pub fn message(message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(message.clone(), json!({ "message": message }))
    }

    pub fn compact(mut self, compact: impl Into<String>) -> Self {
        self.compact = Some(compact.into());
        self
    }

    pub fn markdown(mut self, markdown: impl Into<String>) -> Self {
        self.markdown = Some(markdown.into());
        self
    }

    pub fn print(&self, mode: OutputMode) -> Result<()> {
        match mode {
            OutputMode::Default => print_non_empty(&self.default),
            OutputMode::Json => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&self.value)
                        .context("serialize beam json output")?
                );
            }
            OutputMode::Yaml => {
                print_non_empty(
                    serde_yaml::to_string(&self.value).context("serialize beam yaml output")?,
                );
            }
            OutputMode::Markdown => {
                print_non_empty(
                    self.markdown
                        .clone()
                        .unwrap_or_else(|| self.default.clone()),
                );
            }
            OutputMode::Compact => {
                print_non_empty(self.compact.clone().unwrap_or_else(|| self.default.clone()));
            }
            OutputMode::Quiet => {}
        }

        Ok(())
    }
}

impl OutputMode {
    pub(crate) fn shows_loading(self) -> bool {
        should_render_loading(self, std::io::stderr().is_terminal())
    }
}

pub async fn with_loading<T, F>(
    mode: OutputMode,
    message: impl Into<String>,
    future: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    with_loading_interrupt(mode, message, future, tokio::signal::ctrl_c()).await
}

pub(crate) async fn with_loading_interrupt<T, F, C>(
    mode: OutputMode,
    message: impl Into<String>,
    future: F,
    cancel: C,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
    C: Future<Output = std::io::Result<()>>,
{
    let indicator = LoadingIndicator::start(mode, message.into());
    let output = with_interrupt(future, cancel).await;

    drop(indicator);
    output
}

pub(crate) async fn with_interrupt<T, F, C>(future: F, cancel: C) -> Result<T>
where
    F: Future<Output = Result<T>>,
    C: Future<Output = std::io::Result<()>>,
{
    let mut future = std::pin::pin!(future);
    let mut cancel = std::pin::pin!(cancel);
    tokio::select! {
        output = &mut future => output,
        signal = &mut cancel => {
            signal.context("listen for beam ctrl-c")?;
            Err(Error::Interrupted)
        }
    }
}

pub async fn with_loading_handle<T, F, Fut>(
    mode: OutputMode,
    message: impl Into<String>,
    run: F,
) -> T
where
    F: FnOnce(LoadingHandle) -> Fut,
    Fut: Future<Output = T>,
{
    let indicator = LoadingIndicator::start(mode, message.into());
    let output = run(indicator.handle()).await;
    drop(indicator);
    output
}

pub fn confirmed_transaction_message(
    summary: impl Into<String>,
    tx_hash: &str,
    block_number: Option<u64>,
) -> String {
    let block = block_number.map_or_else(|| "unknown".to_string(), |value| value.to_string());

    format!("{}\nTx: {tx_hash}\nBlock: {block}", summary.into())
}

pub fn pending_transaction_message(
    summary: impl Into<String>,
    tx_hash: &str,
    block_number: Option<u64>,
) -> String {
    let block = block_number.map_or_else(|| "pending".to_string(), |value| value.to_string());

    format!("{}\nTx: {tx_hash}\nBlock: {block}", summary.into())
}

pub fn dropped_transaction_message(
    summary: impl Into<String>,
    tx_hash: &str,
    block_number: Option<u64>,
) -> String {
    let block = block_number.map_or_else(|| "pending".to_string(), |value| value.to_string());

    format!(
        "{}\nTx: {tx_hash}\nLast seen block: {block}",
        summary.into()
    )
}

pub fn balance_message(summary: impl Into<String>, address: &str) -> String {
    format!("{}\nAddress: {address}", summary.into())
}

pub(crate) fn should_render_loading(mode: OutputMode, is_terminal: bool) -> bool {
    mode == OutputMode::Default && is_terminal
}

fn print_non_empty(text: impl AsRef<str>) {
    let text = text.as_ref();
    if !text.is_empty() {
        println!("{text}");
    }
}

struct LoadingIndicator {
    handle: Option<JoinHandle<()>>,
    control_tx: Option<mpsc::Sender<LoadingControl>>,
}

impl LoadingIndicator {
    fn disabled() -> Self {
        Self {
            handle: None,
            control_tx: None,
        }
    }

    fn handle(&self) -> LoadingHandle {
        LoadingHandle {
            control_tx: self.control_tx.clone(),
        }
    }

    fn start(mode: OutputMode, message: String) -> Self {
        if !mode.shows_loading() {
            return Self::disabled();
        }

        let (control_tx, control_rx) = mpsc::channel::<LoadingControl>();
        let handle = thread::spawn(move || render_loading(control_rx, message));

        Self {
            handle: Some(handle),
            control_tx: Some(control_tx),
        }
    }
}

impl Drop for LoadingIndicator {
    fn drop(&mut self) {
        if let Some(control_tx) = self.control_tx.take() {
            let _ = control_tx.send(LoadingControl::Stop);
        }

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl LoadingHandle {
    pub fn set_message(&self, message: impl Into<String>) {
        if let Some(control_tx) = &self.control_tx {
            let _ = control_tx.send(LoadingControl::Update(message.into()));
        }
    }
}

enum LoadingControl {
    Stop,
    Update(String),
}

fn render_loading(control_rx: mpsc::Receiver<LoadingControl>, mut message: String) {
    let mut stderr = std::io::stderr().lock();
    let mut frame_index = 0usize;
    let mut line_width = 0usize;

    loop {
        let frame = LOADER_FRAMES[frame_index % LOADER_FRAMES.len()];
        let line = format!("{frame} {message}");
        let padding = line_width.saturating_sub(line.len());
        line_width = line.len();

        let _ = write!(stderr, "\r{line}{}", " ".repeat(padding));
        let _ = stderr.flush();

        match control_rx.recv_timeout(LOADER_TICK) {
            Ok(LoadingControl::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Ok(LoadingControl::Update(next_message)) => message = next_message,
            Err(RecvTimeoutError::Timeout) => frame_index += 1,
        }
    }

    if line_width > 0 {
        let _ = write!(stderr, "\r{}\r", " ".repeat(line_width));
        let _ = stderr.flush();
    }
}
