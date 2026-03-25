mod error;
mod git;
mod lint;
mod noir_fixtures;
mod revi;
mod setup;
mod test;

use clap::{Parser, Subcommand};

use crate::error::Result;
use crate::lint::{LintArgs, run_lint};
use crate::noir_fixtures::{NoirFixturesArgs, run_noir_fixtures};
use crate::revi::{ReviArgs, run_revi};
use crate::setup::{SetupArgs, run_setup};
use crate::test::{TestArgs, run_test};

#[derive(Parser)]
#[command(author = None, version = env!("CARGO_PKG_VERSION"), about = "Developer automation tasks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all linting tooling with optional auto-fixes
    Lint(LintArgs),
    /// Generate Noir circuit fixtures (program, key, key_fields) from workspace bins
    NoirFixtures(NoirFixturesArgs),
    /// Prepare the local development environment and print export commands
    Setup(SetupArgs),
    /// Run tests for changed crates and their dependents
    Test(TestArgs),
    /// Run revi with the remaining arguments
    Revi(ReviArgs),
}

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Lint(args) => run_lint(args),
        Commands::NoirFixtures(args) => run_noir_fixtures(args),
        Commands::Revi(args) => run_revi(args),
        Commands::Setup(args) => run_setup(args),
        Commands::Test(args) => run_test(args),
    }
}
