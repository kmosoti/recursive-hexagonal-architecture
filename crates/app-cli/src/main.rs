//! `rhawiki`: the composition root. Adapters are constructed here and only
//! here (AGENTS.md).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use site::CheckWitness;

#[derive(Debug, Parser)]
#[command(
    name = "rhawiki",
    about = "A research wiki that renders a directory of markdown"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Render every page under --root to HTML under --out.
    Build {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Report witnesses; exit 1 if there are any.
    Check {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
}

fn load(root: PathBuf) -> Result<(library::Corpus, Vec<library::DuplicatePageId>), String> {
    library::load(&adapter_fs::FsSources::new(root)).map_err(|e| format!("loading failed: {e:?}"))
}

fn json(w: &CheckWitness) -> serde_json::Value {
    match w {
        CheckWitness::DuplicatePageId { id } => {
            serde_json::json!({"kind": "duplicate_page_id", "id": id})
        }
        CheckWitness::DuplicateSlug { page, slug } => {
            serde_json::json!({"kind": "duplicate_slug", "page": page, "slug": slug})
        }
        CheckWitness::BrokenLink { from, target } => {
            serde_json::json!({"kind": "broken_link", "from": from, "target": target})
        }
        CheckWitness::AmbiguousLink { from, target } => {
            serde_json::json!({"kind": "ambiguous_link", "from": from, "target": target})
        }
        CheckWitness::MissingAnchor {
            from,
            target,
            heading,
        } => {
            serde_json::json!({"kind": "missing_anchor", "from": from, "target": target, "heading": heading})
        }
    }
}

fn run(cli: Cli) -> Result<u8, String> {
    match cli.command {
        Command::Build { root, out } => {
            let (corpus, _) = load(root)?;
            let mut sink = adapter_fs::FsSink::new(out);
            let report = site::build_all(
                &corpus,
                &adapter_sys::SystemClock,
                &mut sink,
                &adapter_html::HtmlRenderer,
            )
            .map_err(|e| format!("build failed: {e:?}"))?;
            println!(
                "built {} page(s): {} written, {} deleted, {} unchanged, {} link witness(es)",
                corpus.len(),
                report.written.len(),
                report.deleted.len(),
                report.unchanged,
                report.link_witnesses.len()
            );
            Ok(0)
        }
        Command::Check { root, format } => {
            let (corpus, duplicates) = load(root)?;
            let (documents, graph) = site::analyse(&corpus);
            let witnesses = site::witnesses(&duplicates, &documents, &graph);
            let rows: Vec<serde_json::Value> = witnesses.iter().map(json).collect();
            if format == "json" {
                let mut counts = serde_json::Map::new();
                for r in &rows {
                    let k = r["kind"].as_str().unwrap_or_default().to_owned();
                    let n = counts
                        .get(&k)
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0)
                        + 1;
                    counts.insert(k, n.into());
                }
                let doc = serde_json::json!({"schema_version": 1, "pages": corpus.len(), "witnesses": rows, "counts": counts});
                println!(
                    "{}",
                    serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?
                );
            } else {
                for r in &rows {
                    println!("{r}");
                }
                println!("{} page(s), {} witness(es)", corpus.len(), rows.len());
            }
            Ok(u8::from(!rows.is_empty()))
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("rhawiki: {message}");
            ExitCode::from(2)
        }
    }
}
