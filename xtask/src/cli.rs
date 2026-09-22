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
    /// Run the pre-registered H4 corpus (crate level; modules arrive in W7).
    Corpus {
        #[command(subcommand)]
        command: CorpusCommand,
    },
    /// Evidence utilities.
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// Check that the change stays inside its task record's scope and the
    /// repository layout (plan §2.2 M4).
    Scope {
        /// The task record id, for example CHG-005.
        #[arg(long)]
        task: String,
        /// The revision the change is compared with, by merge base.
        #[arg(long, default_value = "origin/main")]
        base: String,
    },
    /// Write the generated docs, or with --check report stale ones.
    Docs {
        /// Compare instead of writing; exit 1 if any generated file is stale.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum CorpusCommand {
    /// Materialize the declared fixtures, or fail if committed inputs drift.
    Generate {
        #[arg(long)]
        check: bool,
    },
    /// Check committed fixtures, grade every case and write H4 evidence.
    Run(CorpusArgs),
    /// Run the accepted checker on Kennedy's private cases without any agent
    /// reading them (DP-1.1b as amended in CHG-004.6). Prints opaque ids,
    /// exit statuses and counts; grades nothing.
    HeldOut(HeldOutArgs),
}

#[derive(Debug, Args)]
pub struct HeldOutArgs {
    /// A directory of ready workspaces, each holding Cargo.toml and
    /// rha-crates.toml; verified against the DP-1.1b commitment by
    /// re-creating the committed tar stream.
    #[arg(long, conflicts_with = "archive")]
    pub cases: Option<PathBuf>,
    /// The original committed tar archive; verified by its bytes, then
    /// extracted into the private directory.
    #[arg(long)]
    pub archive: Option<PathBuf>,
    /// `held_out` refuses a mismatched archive; `public_control` exercises
    /// the runner on public fixtures.
    #[arg(long, default_value = "held_out", value_parser = ["held_out", "public_control"])]
    pub purpose: String,
    /// Where raw reports and the private case map go. Defaults to
    /// $XDG_STATE_HOME/rha/held-out, else ~/.local/state/rha/held-out.
    #[arg(long)]
    pub private_dir: Option<PathBuf>,
    /// `architecture`: each workspace with Cargo.toml and rha-crates.toml
    /// runs `cargo xtask architecture` (DP-1.1b). `check`: each immediate
    /// subdirectory is a markdown site and runs `rhawiki check` (DP-1.4).
    #[arg(long, default_value = "architecture", value_parser = ["architecture", "check"])]
    pub kind: String,
}

#[derive(Debug, Args)]
pub struct CorpusArgs {
    #[arg(long, default_value = "crate", value_parser = ["crate", "module", "markdown"])]
    pub level: String,
    /// Directory for the timestamped advisory H4 record.
    #[arg(long, default_value = "evidence/h4-crate")]
    pub evidence: PathBuf,
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
    /// Accepted for the L2 interface. This version has no transitive
    /// evaluator: the flag prints a notice, and [transitive] enabled = true
    /// in the rules file is refused with exit 2.
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
