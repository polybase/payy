use crate::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapArgs {
    pub amount: String,
    pub buy_token: String,
    pub deadline_seconds: u64,
    pub max_gas: Option<String>,
    pub min_receive: Option<String>,
    pub recipient: Option<String>,
    pub sell_token: String,
    pub slippage_bps: u32,
    pub unlimited_approval: bool,
}

impl SwapArgs {
    pub fn parse(args: &[String]) -> Result<Self> {
        if args.len() < 4 || args.first().map(String::as_str) != Some("swap") {
            return Err(Error::UnsupportedCommand {
                command: "swap requires sell token, buy token, and amount".to_string(),
            });
        }

        let mut output = Self {
            amount: args[3].clone(),
            buy_token: args[2].clone(),
            deadline_seconds: 20 * 60,
            max_gas: None,
            min_receive: None,
            recipient: None,
            sell_token: args[1].clone(),
            slippage_bps: 50,
            unlimited_approval: false,
        };

        let mut index = 4;
        while index < args.len() {
            match args[index].as_str() {
                "--deadline-seconds" => {
                    output.deadline_seconds = parse_next(args, &mut index, "--deadline-seconds")?
                        .parse::<u64>()
                        .map_err(|_| Error::InvalidArgument {
                            reason: "invalid deadline seconds".to_string(),
                        })?;
                }
                "--max-gas" => output.max_gas = Some(parse_next(args, &mut index, "--max-gas")?),
                "--min-receive" => {
                    output.min_receive = Some(parse_next(args, &mut index, "--min-receive")?)
                }
                "--recipient" => {
                    output.recipient = Some(parse_next(args, &mut index, "--recipient")?)
                }
                "--slippage" => {
                    output.slippage_bps =
                        parse_slippage_bps(&parse_next(args, &mut index, "--slippage")?)?;
                }
                "--slippage-bps" => {
                    output.slippage_bps = parse_next(args, &mut index, "--slippage-bps")?
                        .parse::<u32>()
                        .map_err(|_| Error::InvalidArgument {
                            reason: "invalid slippage bps".to_string(),
                        })?;
                }
                "--unlimited-approval" => output.unlimited_approval = true,
                other => {
                    return Err(Error::InvalidArgument {
                        reason: format!("unsupported uniswap flag {other}"),
                    });
                }
            }
            index += 1;
        }

        Ok(output)
    }
}

fn parse_next(args: &[String], index: &mut usize, flag: &str) -> Result<String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| Error::InvalidArgument {
            reason: format!("{flag} requires a value"),
        })
}

fn parse_slippage_bps(value: &str) -> Result<u32> {
    let Some((whole, fractional)) = value.split_once('.') else {
        return value
            .parse::<u32>()
            .map(|value| value * 100)
            .map_err(|_| Error::InvalidArgument {
                reason: "invalid slippage percent".to_string(),
            });
    };
    let whole = whole.parse::<u32>().map_err(|_| Error::InvalidArgument {
        reason: "invalid slippage percent".to_string(),
    })?;
    let mut fractional = fractional.chars().take(2).collect::<String>();
    while fractional.len() < 2 {
        fractional.push('0');
    }
    let fractional = fractional
        .parse::<u32>()
        .map_err(|_| Error::InvalidArgument {
            reason: "invalid slippage percent".to_string(),
        })?;
    Ok(whole * 100 + fractional)
}
