use crate::display::{
    ColorMode, error_message, render_colored_shell_prefix, render_shell_prefix, should_color,
    shrink, warning_message,
};

#[test]
fn auto_color_only_enables_for_real_terminals() {
    assert!(should_color(ColorMode::Auto, true, false, false));
    assert!(!should_color(ColorMode::Auto, false, false, false));
    assert!(!should_color(ColorMode::Auto, true, true, false));
    assert!(!should_color(ColorMode::Auto, true, false, true));
    assert!(should_color(ColorMode::Always, false, true, true));
    assert!(!should_color(ColorMode::Never, true, false, false));
}

#[test]
fn shell_prefix_stays_plain_when_color_is_disabled() {
    let prefix = render_shell_prefix(
        "wallet-1 0x740747e7...e3a1e112",
        "ethereum",
        "https://et...node.com",
    );

    assert_eq!(
        prefix,
        "[wallet-1 0x740747e7...e3a1e112 | ethereum | https://et...node.com] "
    );
}

#[test]
fn shell_prefix_uses_brand_colors_for_known_chains() {
    let wallet_display = "wallet-1 0x740747e7...e3a1e112";
    let rpc_url = "https://et...node.com";
    let cases = [
        ("ethereum", "\x1b[1;38;2;98;126;234methereum\x1b[0m"),
        ("polygon", "\x1b[1;38;2;130;71;229mpolygon\x1b[0m"),
        ("bnb", "\x1b[1;38;2;243;186;47mbnb\x1b[0m"),
        ("payy-testnet", "\x1b[1;38;2;224;255;50mpayy-testnet\x1b[0m"),
    ];

    for (chain, expected_fragment) in cases {
        let prefix = render_colored_shell_prefix(wallet_display, chain, rpc_url);

        assert!(prefix.contains("\x1b[1;36mwallet-1 0x740747e7...e3a1e112\x1b[0m"));
        assert!(prefix.contains(expected_fragment));
        assert!(prefix.contains("\x1b[1;34mhttps://et...node.com\x1b[0m"));
        assert!(!prefix.contains('\x01'));
        assert!(!prefix.contains('\x02'));
        assert!(prefix.ends_with(' '));
    }
}

#[test]
fn shell_prefix_falls_back_to_the_default_prompt_chain_color_for_unknown_networks() {
    let prefix = render_colored_shell_prefix(
        "wallet-1 0x740747e7...e3a1e112",
        "beam-dev",
        "https://et...node.com",
    );

    assert!(prefix.contains("\x1b[1;33mbeam-dev\x1b[0m"));
}

#[test]
fn shell_prefix_sanitizes_control_characters_in_dynamic_segments() {
    let plain = render_shell_prefix("ali\nce \x1b[31m", "beam-\x1b[32m", "https://rpc/\x1b[0m");
    assert_eq!(plain, "[ali ce ?[31m | beam-?[32m | https://rpc/?[0m] ");

    let colored =
        render_colored_shell_prefix("ali\nce \x1b[31m", "beam-\x1b[32m", "https://rpc/\x1b[0m");
    assert!(colored.contains("ali ce ?[31m"));
    assert!(colored.contains("beam-?[32m"));
    assert!(colored.contains("https://rpc/?[0m"));
}

#[test]
fn shrink_truncates_utf8_values_on_character_boundaries() {
    let url = "https://例え.example/路径/交易/éééééééé";

    assert_eq!(shrink(url), "https://例え...éééééééé");
}

#[test]
fn label_messages_only_colorize_the_prefix() {
    assert_eq!(
        warning_message("beam 9.9.9 is available.", false),
        "Warning: beam 9.9.9 is available."
    );
    assert_eq!(error_message("boom", false), "Error: boom");

    let colored = error_message("boom", true);

    assert!(colored.starts_with("\x1b[1;31mError:\x1b[0m "));
    assert!(colored.ends_with("boom"));
}

#[test]
fn label_messages_sanitize_control_characters_in_message_body() {
    assert_eq!(
        warning_message("beam\n9.9.9\t\x1b[31m", false),
        "Warning: beam 9.9.9 ?[31m"
    );
}
