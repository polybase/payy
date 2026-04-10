use std::io::{BufRead, Write};

use contextful::ResultContextExt;

use crate::{
    chains::BeamChains,
    cli::FetchArgs,
    error::{Error, Result},
    human_output::sanitize_control_chars,
};

use super::PreparedPayment;

pub(crate) fn approve_payment(
    args: &FetchArgs,
    payment: &PreparedPayment,
    chain_store: &BeamChains,
) -> Result<()> {
    let stdin = std::io::stdin();
    let stderr = std::io::stderr();

    approve_payment_with(
        args,
        payment,
        chain_store,
        &mut stdin.lock(),
        &mut stderr.lock(),
    )
}

pub(crate) fn approve_payment_with<R, W>(
    args: &FetchArgs,
    payment: &PreparedPayment,
    chain_store: &BeamChains,
    input: &mut R,
    output: &mut W,
) -> Result<()>
where
    R: BufRead,
    W: Write,
{
    ensure_payment_chain_allowed(args, payment, chain_store, input, output)?;

    if let Some(max_fee) = args.max_fee.as_ref() {
        payment.ensure_max_fee_allows(max_fee)?;
        return Ok(());
    }

    if confirm_with(input, output, "Pay now? [y/N]: ")? {
        Ok(())
    } else {
        Err(Error::FetchPaymentRejected)
    }
}

fn ensure_payment_chain_allowed<R, W>(
    args: &FetchArgs,
    payment: &PreparedPayment,
    chain_store: &BeamChains,
    input: &mut R,
    output: &mut W,
) -> Result<()>
where
    R: BufRead,
    W: Write,
{
    if !args.allowed_chains.is_empty() {
        if args
            .allowed_chains
            .iter()
            .any(|selector| payment.chain.matches_selector(selector, chain_store))
        {
            return Ok(());
        }

        return Err(Error::FetchPaymentChainNotAllowed {
            chain: payment.chain.summary(),
        });
    }

    let Some(selected_chain) = payment.selected_chain.as_ref() else {
        return Ok(());
    };
    if selected_chain.chain_id == payment.chain.chain_id {
        return Ok(());
    }

    let prompt = format!(
        "Accept payment request on {} instead of selected chain {}? [y/N]: ",
        sanitize_control_chars(&payment.chain.summary()),
        sanitize_control_chars(&selected_chain.summary()),
    );

    if confirm_with(input, output, &prompt)? {
        Ok(())
    } else {
        Err(Error::FetchPaymentRejected)
    }
}

fn confirm_with<R, W>(input: &mut R, output: &mut W, prompt: &str) -> Result<bool>
where
    R: BufRead,
    W: Write,
{
    loop {
        write!(output, "{prompt}").context("write beam fetch prompt")?;
        output.flush().context("flush beam fetch prompt")?;

        let mut value = String::new();
        if input
            .read_line(&mut value)
            .context("read beam fetch prompt")?
            == 0
        {
            return Err(Error::PromptClosed {
                label: "beam fetch payment".to_string(),
            });
        }

        match value.trim().to_ascii_lowercase().as_str() {
            "" | "n" | "no" => return Ok(false),
            "y" | "yes" => return Ok(true),
            _ => continue,
        }
    }
}
