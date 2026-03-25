use std::io::{IsTerminal, Read};

use contextful::ResultContextExt;

use crate::error::{Error, Result};

pub mod abi;
mod abi_topic;
pub mod bytes;
pub mod hash;
pub mod numbers;
pub mod rlp;

pub fn value_or_stdin_text(value: Option<String>, command: &str) -> Result<String> {
    match value {
        Some(value) => Ok(value),
        None => read_stdin_text(command),
    }
}

pub fn value_or_stdin_bytes(value: Option<String>, command: &str) -> Result<Vec<u8>> {
    match value {
        Some(value) => Ok(value.into_bytes()),
        None => read_stdin_bytes(command),
    }
}

fn read_stdin_text(command: &str) -> Result<String> {
    if std::io::stdin().is_terminal() {
        return Err(Error::MissingUtilInput {
            command: command.to_string(),
        });
    }

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("read beam util stdin text")?;
    Ok(input)
}

fn read_stdin_bytes(command: &str) -> Result<Vec<u8>> {
    if std::io::stdin().is_terminal() {
        return Err(Error::MissingUtilInput {
            command: command.to_string(),
        });
    }

    let mut input = Vec::new();
    std::io::stdin()
        .read_to_end(&mut input)
        .context("read beam util stdin bytes")?;
    Ok(input)
}
