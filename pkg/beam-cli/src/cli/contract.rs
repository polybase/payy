use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum ContractAction {
    /// Show deployed contract summary information for a literal 0x address
    Info(ContractAddressArgs),
    #[command(
        long_about = "Print runtime bytecode from the active RPC. Stdout is pipeable. There is no `code` alias; use `bytecode`."
    )]
    /// Print pipeable runtime bytecode from the active RPC
    Bytecode(ContractBytecodeArgs),
    #[command(
        long_about = "Print the verified ABI from Sourcify for the exact literal 0x address. Beam does not follow proxies automatically; proxy implementation addresses are shown only as tips."
    )]
    /// Print the pipeable verified ABI from Sourcify
    Abi(ContractAddressArgs),
    #[command(
        long_about = "Print verified source information or one pipeable source file from Sourcify for the exact literal 0x address. Beam does not follow proxies automatically. Use `--` before a source path that begins with `-`."
    )]
    /// Print verified source information or one pipeable source file
    Source(ContractSourceArgs),
    #[command(
        long_about = "Export the verified Sourcify source bundle for the exact literal 0x address. Beam does not follow proxies automatically. Use `--` before a destination that begins with `-`."
    )]
    /// Export the verified Sourcify source bundle
    Export(ContractExportArgs),
}

#[derive(Clone, Debug, Args)]
pub struct ContractAddressArgs {
    #[arg(help = "Literal 0x contract address; ENS names and aliases are not accepted")]
    pub address: String,
}

#[derive(Clone, Debug, Args)]
pub struct ContractBytecodeArgs {
    #[arg(help = "Literal 0x contract address; ENS names and aliases are not accepted")]
    pub address: String,
    #[arg(long)]
    pub block: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct ContractSourceArgs {
    #[arg(help = "Literal 0x contract address; ENS names and aliases are not accepted")]
    pub address: String,
    #[arg(help = "Sourcify source path; use `--` before values that begin with `-`")]
    pub source_path: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct ContractExportArgs {
    #[arg(help = "Literal 0x contract address; ENS names and aliases are not accepted")]
    pub address: String,
    #[arg(help = "Export destination directory; use `--` before values that begin with `-`")]
    pub destination: String,
}
