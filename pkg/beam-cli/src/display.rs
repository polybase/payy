use std::{ffi::OsStr, io::IsTerminal};

use clap::ValueEnum;

use crate::human_output::sanitize_control_chars;

const ARBITRUM_CHAIN_COLOR: (u8, u8, u8) = (40, 160, 240);
const BASE_CHAIN_COLOR: (u8, u8, u8) = (0, 82, 255);
const BNB_CHAIN_COLOR: (u8, u8, u8) = (243, 186, 47);
const ETHEREUM_CHAIN_COLOR: (u8, u8, u8) = (98, 126, 234);
const HARDHAT_CHAIN_COLOR: (u8, u8, u8) = (255, 241, 17);
const PAYY_CHAIN_COLOR: (u8, u8, u8) = (224, 255, 50);
const POLYGON_CHAIN_COLOR: (u8, u8, u8) = (130, 71, 229);

pub fn shrink(value: &str) -> String {
    let char_count = value.chars().count();
    if char_count <= 24 {
        return value.to_string();
    }

    let prefix_end = value
        .char_indices()
        .nth(10)
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    let suffix_start = value
        .char_indices()
        .nth(char_count - 8)
        .map(|(index, _)| index)
        .unwrap_or(0);

    format!("{}...{}", &value[..prefix_end], &value[suffix_start..])
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl ColorMode {
    pub(crate) fn colors_stderr(self) -> bool {
        should_color(
            self,
            std::io::stderr().is_terminal(),
            std::env::var_os("NO_COLOR").is_some(),
            std::env::var_os("TERM").as_deref() == Some(OsStr::new("dumb")),
        )
    }

    pub(crate) fn colors_stdout(self) -> bool {
        should_color(
            self,
            std::io::stdout().is_terminal(),
            std::env::var_os("NO_COLOR").is_some(),
            std::env::var_os("TERM").as_deref() == Some(OsStr::new("dumb")),
        )
    }
}

pub(crate) fn should_color(
    mode: ColorMode,
    is_terminal: bool,
    no_color: bool,
    term_is_dumb: bool,
) -> bool {
    match mode {
        ColorMode::Auto => is_terminal && !no_color && !term_is_dumb,
        ColorMode::Always => true,
        ColorMode::Never => false,
    }
}

pub(crate) fn error_message(message: &str, color_enabled: bool) -> String {
    label_message("Error:", Style::Error, message, color_enabled)
}

pub(crate) fn notice_message(message: &str, color_enabled: bool) -> String {
    label_message("Notice:", Style::Notice, message, color_enabled)
}

pub(crate) fn render_shell_prefix(wallet_display: &str, chain: &str, rpc_url: &str) -> String {
    let wallet_display = sanitize_control_chars(wallet_display);
    let chain = sanitize_control_chars(chain);
    let rpc_url = sanitize_control_chars(rpc_url);
    format!("[{wallet_display} | {chain} | {rpc_url}] ")
}

pub(crate) fn render_colored_shell_prefix(
    wallet_display: &str,
    chain: &str,
    rpc_url: &str,
) -> String {
    let wallet_display = sanitize_control_chars(wallet_display);
    let chain = sanitize_control_chars(chain);
    let rpc_url = sanitize_control_chars(rpc_url);
    format!(
        "{}{}{}{}{}{}{} ",
        colorize_prompt("[", Style::PromptFrame),
        colorize_prompt(&wallet_display, Style::PromptWallet),
        colorize_prompt(" | ", Style::PromptFrame),
        colorize_chain_prompt(&chain),
        colorize_prompt(" | ", Style::PromptFrame),
        colorize_prompt(&rpc_url, Style::PromptRpc),
        colorize_prompt("]", Style::PromptFrame),
    )
}

pub(crate) fn warning_message(message: &str, color_enabled: bool) -> String {
    label_message("Warning:", Style::Warning, message, color_enabled)
}

#[derive(Clone, Copy)]
enum Style {
    Error,
    Notice,
    PromptChain,
    PromptFrame,
    PromptRpc,
    PromptWallet,
    Warning,
}

fn ansi_code(style: Style) -> &'static str {
    match style {
        Style::Error => "1;31",
        Style::Notice => "1;36",
        Style::PromptChain => "1;33",
        Style::PromptFrame => "2;37",
        Style::PromptRpc => "1;34",
        Style::PromptWallet => "1;36",
        Style::Warning => "1;33",
    }
}

fn colorize(text: &str, style: Style, color_enabled: bool) -> String {
    colorize_with_ansi_code(text, ansi_code(style), color_enabled)
}

fn colorize_chain_prompt(chain: &str) -> String {
    let chain_ansi_code = prompt_chain_ansi_code(chain);
    colorize_with_ansi_code(chain, chain_ansi_code.as_str(), true)
}

fn colorize_with_ansi_code(text: &str, ansi_code: &str, color_enabled: bool) -> String {
    if !color_enabled {
        return text.to_string();
    }

    format!("\x1b[{ansi_code}m{text}\x1b[0m")
}

fn colorize_prompt(text: &str, style: Style) -> String {
    colorize(text, style, true)
}

fn prompt_chain_ansi_code(chain: &str) -> String {
    let normalized = normalize_chain_key(chain);
    let color = if normalized.starts_with("payy") {
        Some(PAYY_CHAIN_COLOR)
    } else {
        match normalized.as_str() {
            "arbitrum" | "arb" => Some(ARBITRUM_CHAIN_COLOR),
            "base" => Some(BASE_CHAIN_COLOR),
            "bnb" | "bsc" | "binance" => Some(BNB_CHAIN_COLOR),
            "ethereum" | "eth" | "sepolia" => Some(ETHEREUM_CHAIN_COLOR),
            "hardhat" | "local" => Some(HARDHAT_CHAIN_COLOR),
            "polygon" => Some(POLYGON_CHAIN_COLOR),
            _ => None,
        }
    };

    match color {
        Some((red, green, blue)) => format!("1;38;2;{red};{green};{blue}"),
        None => ansi_code(Style::PromptChain).to_string(),
    }
}

fn normalize_chain_key(chain: &str) -> String {
    chain.trim().replace(['_', ' '], "-").to_ascii_lowercase()
}

fn label_message(label: &str, style: Style, message: &str, color_enabled: bool) -> String {
    let message = sanitize_control_chars(message);
    format!("{} {message}", colorize(label, style, color_enabled))
}
