use std::io::{BufRead, Write};

use contextful::ResultContextExt;

use crate::error::{Error, Result};

pub fn prompt_required(label: &str) -> Result<String> {
    let stdin = std::io::stdin();
    let stderr = std::io::stderr();

    prompt_required_with(label, &mut stdin.lock(), &mut stderr.lock())
}

pub fn prompt_with_default(label: &str, default_value: &str) -> Result<String> {
    let stdin = std::io::stdin();
    let stderr = std::io::stderr();

    prompt_with_default_with(label, default_value, &mut stdin.lock(), &mut stderr.lock())
}

pub(crate) fn prompt_required_with<R, W>(
    label: &str,
    input: &mut R,
    output: &mut W,
) -> Result<String>
where
    R: BufRead,
    W: Write,
{
    loop {
        write!(output, "{label}: ").context("write beam prompt")?;
        output.flush().context("flush beam prompt")?;

        let mut value = String::new();
        if input.read_line(&mut value).context("read beam prompt")? == 0 {
            return Err(Error::PromptClosed {
                label: label.to_string(),
            });
        }
        let value = value.trim();

        if !value.is_empty() {
            return Ok(value.to_string());
        }
    }
}

pub(crate) fn prompt_with_default_with<R, W>(
    label: &str,
    default_value: &str,
    input: &mut R,
    output: &mut W,
) -> Result<String>
where
    R: BufRead,
    W: Write,
{
    write!(output, "{label} [{default_value}]: ").context("write beam prompt")?;
    output.flush().context("flush beam prompt")?;

    let mut value = String::new();
    if input.read_line(&mut value).context("read beam prompt")? == 0 {
        return Err(Error::PromptClosed {
            label: label.to_string(),
        });
    }

    match value.trim() {
        "" => Ok(default_value.to_string()),
        value => Ok(value.to_string()),
    }
}
