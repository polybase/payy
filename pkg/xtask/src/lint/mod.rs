// lint-long-file-override allow-max-lines=250
mod i18n;
mod steps;

use std::path::Path;

use clap::{ArgGroup, Args, ValueEnum};

use crate::error::{Result, XTaskError, workspace_root};
use crate::lint::steps::{
    StepResult, print_step, run_ast_grep, run_claude_doc, run_clippy, run_file_length, run_hakari,
    run_i18n_consistency, run_rustfmt, run_taplo_check, run_taplo_fmt, run_workspace_deps,
};
/// Available linter types
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LinterType {
    /// GENERATED_AI_GUIDANCE.md regeneration
    ClaudeGuidelines,
    /// Rust formatter
    Rustfmt,
    /// TOML formatter and checker
    Taplo,
    /// AST-based linting
    AstGrep,
    /// Rust linter
    Clippy,
    /// File length checker
    FileLength,
    /// Internationalization locale consistency
    I18nConsistency,
    /// Cargo Hakari workspace-hack consistency
    Hakari,
    /// Workspace dependency inheritance validator
    WorkspaceDeps,
}
#[derive(Args)]
#[command(group = ArgGroup::new("lint-mode").args(["fix", "check"]).multiple(false))]
pub struct LintArgs {
    /// Apply auto-fixes where available (default behaviour)
    #[arg(long)]
    pub fix: bool,
    /// Check only, do not attempt to fix issues
    #[arg(long)]
    pub check: bool,
    /// Filter specific linters to run (can specify multiple)
    /// If not provided, all linters will run
    #[arg(long, value_enum)]
    pub filter: Option<Vec<LinterType>>,
}

impl LintArgs {
    pub fn mode(&self) -> LintMode {
        if self.check {
            LintMode::CheckOnly
        } else {
            LintMode::AutoFix
        }
    }
}

#[derive(Clone, Copy)]
pub enum LintMode {
    AutoFix,
    CheckOnly,
}

pub fn run_lint(args: LintArgs) -> Result<()> {
    let repo_root = workspace_root()?;
    println!("Running xtask lint...");

    let mode = args.mode();
    let mut results = Vec::new();

    run_sync_linters(&repo_root, mode, &args.filter, &mut results)?;

    summarize_results(&results)
}

fn run_sync_linters(
    repo_root: &Path,
    mode: LintMode,
    filters: &Option<Vec<LinterType>>,
    results: &mut Vec<StepResult>,
) -> Result<()> {
    run_conditional_linter(
        filters,
        results,
        LinterType::ClaudeGuidelines,
        Box::new(|| run_claude_doc(repo_root, mode)),
    )?;
    run_conditional_linter(
        filters,
        results,
        LinterType::Rustfmt,
        Box::new(|| run_rustfmt(repo_root, mode)),
    )?;

    if should_run_linter(filters, LinterType::Taplo) {
        record_result(results, run_taplo_fmt(repo_root, mode)?);
        record_result(results, run_taplo_check(repo_root)?);
    }

    run_conditional_linter(
        filters,
        results,
        LinterType::AstGrep,
        Box::new(|| run_ast_grep(repo_root)),
    )?;
    run_conditional_linter(
        filters,
        results,
        LinterType::FileLength,
        Box::new(|| run_file_length(repo_root)),
    )?;
    run_conditional_linter(
        filters,
        results,
        LinterType::I18nConsistency,
        Box::new(|| run_i18n_consistency(repo_root)),
    )?;
    run_conditional_linter(
        filters,
        results,
        LinterType::WorkspaceDeps,
        Box::new(|| run_workspace_deps(repo_root)),
    )?;
    run_conditional_linter(
        filters,
        results,
        LinterType::Hakari,
        Box::new(|| run_hakari(repo_root, mode)),
    )?;
    run_conditional_linter(
        filters,
        results,
        LinterType::Clippy,
        Box::new(|| run_clippy(repo_root)),
    )?;

    Ok(())
}

fn run_conditional_linter<'a>(
    filters: &Option<Vec<LinterType>>,
    results: &mut Vec<StepResult>,
    linter: LinterType,
    mut run: Box<dyn FnMut() -> Result<StepResult> + 'a>,
) -> Result<()> {
    if should_run_linter(filters, linter) {
        record_result(results, run()?);
    }

    Ok(())
}

fn record_result(results: &mut Vec<StepResult>, step: StepResult) {
    print_step(&step);
    results.push(step);
}

fn summarize_results(results: &[StepResult]) -> Result<()> {
    let failures = results.iter().filter(|result| result.is_failure()).count();

    if failures == 0 {
        println!("Summary: all lint checks passed");
        Ok(())
    } else {
        println!("Summary: {failures} lint check(s) failed");
        Err(XTaskError::LintFailures { count: failures })
    }
}

fn should_run_linter(filters: &Option<Vec<LinterType>>, linter: LinterType) -> bool {
    filters
        .as_ref()
        .map(|filters| filters.contains(&linter))
        .unwrap_or(true)
}
