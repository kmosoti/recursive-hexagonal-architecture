//! Command-line interface of `cargo xtask`.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "xtask",
    about = "Repository automation for RHA (see CONTRIBUTING.md)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run a lane of .rha/policy.toml and write an evidence record.
    Ci(CiArgs),
    /// Crate-graph and module-graph checks (spec §6.13).
    Architecture(ArchitectureArgs),
    /// Evidence utilities.
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// Write the generated docs, or with --check report stale ones.
    Docs {
        /// Compare instead of writing; exit 1 if any generated file is stale.
        #[arg(long)]
        check: bool,
    },
}

/// `cargo xtask architecture` (plan §5).
#[derive(Debug, clap::Args)]
pub struct ArchitectureArgs {
    /// Check an external workspace instead of this one.
    #[arg(long, value_name = "PATH")]
    pub manifest_path: Option<std::path::PathBuf>,
    /// Use an external rules file instead of rha-crates.toml at the root.
    #[arg(long, value_name = "PATH")]
    pub rules: Option<std::path::PathBuf>,
    /// How to render the report on stdout. The JSON report is always written
    /// to target/rha/architecture.json whatever this says.
    #[arg(long, default_value = "text", value_parser = ["text", "json", "md", "markdown"])]
    pub format: String,
    /// Resolve dependencies fully. Reported `not_run` unless the rules file
    /// sets [transitive] enabled.
    #[arg(long)]
    pub transitive: bool,
}

#[derive(Debug, Subcommand)]
pub enum EvidenceCommand {
    /// Print the artifact identity of the working tree as JSON.
    Subject,
}

#[derive(Debug, Clone, Args)]
pub struct CiArgs {
    /// Lane to run, as named in .rha/policy.toml.
    #[arg(long, default_value = "L0")]
    pub lane: String,
    /// Environment label; `ci` records evidence class "ci" and treats a missing tool as a failure of the run.
    #[arg(long, value_enum, default_value_t = Label::Local)]
    pub label: Label,
    /// Also write a timestamped copy of the record into this directory.
    #[arg(long)]
    pub record: Option<PathBuf>,
    /// Run only this check id; the others are recorded `not_run`.
    #[arg(long)]
    pub only: Option<String>,
    /// Print the lane's commands in order and exit.
    #[arg(long)]
    pub print: bool,
    /// Task id (CHG-nnn) whose task record supplies the change claim.
    #[arg(long)]
    pub task: Option<String>,
    /// Principal that produced this record, for example agent:executor.
    #[arg(long)]
    pub principal: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Label {
    Local,
    Ci,
}
