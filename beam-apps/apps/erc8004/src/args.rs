use crate::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Support,
    ConfigShow,
    ConfigSet(ConfigSetArgs),
    Register(RegisterArgs),
    Show(ShowArgs),
    List(ListArgs),
    SetUri(SetUriArgs),
    SetWallet(SetWalletArgs),
    UnsetWallet(UnsetWalletArgs),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigSetArgs {
    pub identity_registry: String,
    pub reputation_registry: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterArgs {
    pub uri: Option<String>,
    pub identity_registry: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShowArgs {
    pub agent_id: String,
    pub fetch_uri: bool,
    pub identity_registry: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListArgs {
    pub connection: ConnectionMode,
    pub from_block: Option<u64>,
    pub identity_registry: Option<String>,
    pub to_block: Option<u64>,
    pub wallet: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetUriArgs {
    pub agent_id: String,
    pub identity_registry: Option<String>,
    pub uri: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetWalletArgs {
    pub agent_id: String,
    pub deadline_seconds: u64,
    pub identity_registry: Option<String>,
    pub wallet: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsetWalletArgs {
    pub agent_id: String,
    pub identity_registry: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectionMode {
    Owner,
    AgentWallet,
    Both,
}

impl ConnectionMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::AgentWallet => "agent-wallet",
            Self::Both => "both",
        }
    }
}

pub fn parse(args: &[String]) -> Result<Command> {
    let command = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| Error::UnsupportedCommand {
            command: "<missing>".to_string(),
        })?;
    match command {
        "support" => Ok(Command::Support),
        "config" => parse_config(args),
        "register" => parse_register(args),
        "show" => parse_show(args),
        "list" => parse_list(args),
        "set-uri" => parse_set_uri(args),
        "set-wallet" => parse_set_wallet(args),
        "unset-wallet" => parse_unset_wallet(args),
        other => Err(Error::UnsupportedCommand {
            command: other.to_string(),
        }),
    }
}

fn parse_config(args: &[String]) -> Result<Command> {
    match args.get(1).map(String::as_str) {
        Some("show") => Ok(Command::ConfigShow),
        Some("set") => {
            let mut identity_registry = None;
            let mut reputation_registry = None;
            let mut index = 2;
            while index < args.len() {
                match args[index].as_str() {
                    "--identity-registry" => {
                        identity_registry =
                            Some(parse_next(args, &mut index, "--identity-registry")?)
                    }
                    "--reputation-registry" => {
                        reputation_registry =
                            Some(parse_next(args, &mut index, "--reputation-registry")?)
                    }
                    other => return unsupported_flag(other),
                }
                index += 1;
            }
            Ok(Command::ConfigSet(ConfigSetArgs {
                identity_registry: identity_registry.ok_or_else(|| Error::InvalidArgument {
                    reason: "config set requires --identity-registry".to_string(),
                })?,
                reputation_registry,
            }))
        }
        other => Err(Error::UnsupportedCommand {
            command: format!("config {}", other.unwrap_or("<missing>")),
        }),
    }
}

fn parse_register(args: &[String]) -> Result<Command> {
    let mut uri = None;
    let mut empty_uri = false;
    let mut identity_registry = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--uri" => uri = Some(parse_next(args, &mut index, "--uri")?),
            "--empty-uri" => empty_uri = true,
            "--identity-registry" => {
                identity_registry = Some(parse_next(args, &mut index, "--identity-registry")?)
            }
            other => return unsupported_flag(other),
        }
        index += 1;
    }
    if uri.is_some() && empty_uri {
        return Err(Error::InvalidArgument {
            reason: "register accepts either --uri or --empty-uri".to_string(),
        });
    }
    Ok(Command::Register(RegisterArgs {
        uri: if empty_uri { None } else { uri },
        identity_registry,
    }))
}

fn parse_show(args: &[String]) -> Result<Command> {
    let agent_id = args.get(1).cloned().ok_or_else(|| Error::InvalidArgument {
        reason: "show requires <agent-id>".to_string(),
    })?;
    let mut fetch_uri = false;
    let mut identity_registry = None;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--fetch-uri" => fetch_uri = true,
            "--identity-registry" => {
                identity_registry = Some(parse_next(args, &mut index, "--identity-registry")?)
            }
            other => return unsupported_flag(other),
        }
        index += 1;
    }
    Ok(Command::Show(ShowArgs {
        agent_id,
        fetch_uri,
        identity_registry,
    }))
}

fn parse_list(args: &[String]) -> Result<Command> {
    let mut connection = ConnectionMode::Owner;
    let mut from_block = None;
    let mut identity_registry = None;
    let mut to_block = None;
    let mut wallet = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--connection" => {
                connection = match parse_next(args, &mut index, "--connection")?.as_str() {
                    "owner" => ConnectionMode::Owner,
                    "agent-wallet" => ConnectionMode::AgentWallet,
                    "both" => ConnectionMode::Both,
                    value => {
                        return Err(Error::InvalidArgument {
                            reason: format!("invalid connection mode {value}"),
                        });
                    }
                }
            }
            "--from-block" => from_block = Some(parse_u64_flag(args, &mut index, "--from-block")?),
            "--identity-registry" => {
                identity_registry = Some(parse_next(args, &mut index, "--identity-registry")?)
            }
            "--to-block" => to_block = Some(parse_u64_flag(args, &mut index, "--to-block")?),
            "--wallet" => wallet = Some(parse_next(args, &mut index, "--wallet")?),
            other => return unsupported_flag(other),
        }
        index += 1;
    }
    Ok(Command::List(ListArgs {
        connection,
        from_block,
        identity_registry,
        to_block,
        wallet,
    }))
}

fn parse_set_uri(args: &[String]) -> Result<Command> {
    let agent_id = args.get(1).cloned().ok_or_else(|| Error::InvalidArgument {
        reason: "set-uri requires <agent-id>".to_string(),
    })?;
    let uri = args.get(2).cloned().ok_or_else(|| Error::InvalidArgument {
        reason: "set-uri requires <uri>".to_string(),
    })?;
    let mut identity_registry = None;
    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            "--identity-registry" => {
                identity_registry = Some(parse_next(args, &mut index, "--identity-registry")?)
            }
            other => return unsupported_flag(other),
        }
        index += 1;
    }
    Ok(Command::SetUri(SetUriArgs {
        agent_id,
        identity_registry,
        uri,
    }))
}

fn parse_set_wallet(args: &[String]) -> Result<Command> {
    let agent_id = args.get(1).cloned().ok_or_else(|| Error::InvalidArgument {
        reason: "set-wallet requires <agent-id>".to_string(),
    })?;
    let wallet = args.get(2).cloned().ok_or_else(|| Error::InvalidArgument {
        reason: "set-wallet requires <wallet>".to_string(),
    })?;
    let mut deadline_seconds = 300;
    let mut identity_registry = None;
    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            "--deadline-seconds" => {
                deadline_seconds = parse_u64_flag(args, &mut index, "--deadline-seconds")?;
            }
            "--identity-registry" => {
                identity_registry = Some(parse_next(args, &mut index, "--identity-registry")?)
            }
            other => return unsupported_flag(other),
        }
        index += 1;
    }
    if deadline_seconds > 300 {
        return Err(Error::InvalidArgument {
            reason: "set-wallet deadline cannot exceed 300 seconds".to_string(),
        });
    }
    Ok(Command::SetWallet(SetWalletArgs {
        agent_id,
        deadline_seconds,
        identity_registry,
        wallet,
    }))
}

fn parse_unset_wallet(args: &[String]) -> Result<Command> {
    let agent_id = args.get(1).cloned().ok_or_else(|| Error::InvalidArgument {
        reason: "unset-wallet requires <agent-id>".to_string(),
    })?;
    let mut identity_registry = None;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--identity-registry" => {
                identity_registry = Some(parse_next(args, &mut index, "--identity-registry")?)
            }
            other => return unsupported_flag(other),
        }
        index += 1;
    }
    Ok(Command::UnsetWallet(UnsetWalletArgs {
        agent_id,
        identity_registry,
    }))
}

fn parse_next(args: &[String], index: &mut usize, flag: &str) -> Result<String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| Error::InvalidArgument {
            reason: format!("{flag} requires a value"),
        })
}

fn parse_u64_flag(args: &[String], index: &mut usize, flag: &str) -> Result<u64> {
    parse_next(args, index, flag)?
        .parse::<u64>()
        .map_err(|_| Error::InvalidArgument {
            reason: format!("{flag} must be an integer"),
        })
}

fn unsupported_flag<T>(flag: &str) -> Result<T> {
    Err(Error::InvalidArgument {
        reason: format!("unsupported erc8004 flag {flag}"),
    })
}
