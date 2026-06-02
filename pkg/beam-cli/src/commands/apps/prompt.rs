use std::io::{BufRead, Write};

use contextful::ResultContextExt;

use crate::{
    apps::Error as AppError,
    error::{Error, Result},
};

pub(super) fn approve_interactively(summary: &str) -> Result<()> {
    let stdin = std::io::stdin();
    let stderr = std::io::stderr();
    approve_interactively_with(summary, &mut stdin.lock(), &mut stderr.lock())
}

fn approve_interactively_with<R, W>(summary: &str, input: &mut R, output: &mut W) -> Result<()>
where
    R: BufRead,
    W: Write,
{
    writeln!(output, "{summary}").context("write beam app approval summary")?;
    write!(output, "Approve? [y/N]: ").context("write beam app approval prompt")?;
    output.flush().context("flush beam app approval prompt")?;
    let mut value = String::new();
    if input
        .read_line(&mut value)
        .context("read beam app approval prompt")?
        == 0
    {
        return Err(Error::PromptClosed {
            label: "beam app approval".to_string(),
        });
    }
    match value.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => Err(AppError::ApprovalRejected.into()),
    }
}
