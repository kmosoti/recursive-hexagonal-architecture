use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser as _;

use xtask::cli::{Cli, Command, CorpusCommand, EvidenceCommand, RhaCommand};
use xtask::error::Result;
use xtask::{architecture, docs, evidence, lanes};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = workspace_root();
    match dispatch(&root, &cli.command) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

/// The workspace root: the parent of this crate's manifest directory.
fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap_or(manifest_dir).to_path_buf()
}

fn dispatch(root: &Path, command: &Command) -> Result<u8> {
    match command {
        Command::Ci(args) => lanes::run(root, args),
        Command::Corpus {
            command: CorpusCommand::Run(args),
        } => xtask::corpus::runner::run(root, args),
        Command::Corpus {
            command: CorpusCommand::Generate { check },
        } => xtask::corpus::fixture::sync(root, *check),
        Command::Corpus {
            command: CorpusCommand::HeldOut(args),
        } => xtask::corpus::held_out::run(root, args),
        Command::Architecture(args) => {
            let options = architecture::Options {
                manifest_path: args.manifest_path.clone(),
                rules_path: args.rules.clone(),
                format: architecture::Format::parse(&args.format).unwrap_or_default(),
                transitive: args.transitive,
            };
            let code = architecture::run(root, &options)?;
            Ok(u8::try_from(code).unwrap_or(2))
        }
        Command::Evidence {
            command: EvidenceCommand::Subject,
        } => {
            let subject = evidence::subject::capture(root, &root.join(lanes::RUN_DIR))?;
            let text = serde_json::to_string_pretty(&subject)
                .map_err(|e| xtask::error::Error::new(format!("serializing the subject: {e}")))?;
            println!("{text}");
            Ok(0)
        }
        Command::Docs { check } => docs::run(root, *check),
        Command::Rha {
            command: RhaCommand::Lint { files },
        } => xtask::record_lint::run(root, files),
        Command::Scope { task, base } => xtask::scope::run(root, task, base),
    }
}
