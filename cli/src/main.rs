use std::{error::Error, fs, path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand, ValueEnum};
use extendash_core::{
    layout,
    schema::{Document, ExtensionsFile, InboxEvent, Snapshot, WorkspaceManifest},
    SCHEMA_VERSION,
};

#[derive(Parser)]
#[command(name = "extendash", version, about = "ExtenDash command-line frontend")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a workspace data file and check its schemaVersion.
    Check {
        #[arg(value_enum)]
        kind: Kind,
        file: PathBuf,
    },
    /// Print the workspace folder layout.
    Layout,
}

#[derive(Clone, Copy, ValueEnum)]
enum Kind {
    Manifest,
    Extensions,
    Snapshot,
    Inbox,
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<(), Box<dyn Error>> {
    match command {
        Command::Check { kind, file } => {
            let bytes = fs::read(&file)?;
            let summary = match kind {
                Kind::Manifest => {
                    let manifest = WorkspaceManifest::from_json(&bytes)?;
                    format!("workspace {}", manifest.workspace_id)
                }
                Kind::Extensions => {
                    let installed = ExtensionsFile::from_json(&bytes)?;
                    format!("{} extension(s)", installed.extensions.len())
                }
                Kind::Snapshot => {
                    let snapshot = Snapshot::from_json(&bytes)?;
                    format!(
                        "source {}, {} record(s)",
                        snapshot.source_id,
                        snapshot.records.len()
                    )
                }
                Kind::Inbox => {
                    let event = InboxEvent::from_json(&bytes)?;
                    format!(
                        "source {} from {}, {} record(s)",
                        event.source_id,
                        event.origin,
                        event.records.len()
                    )
                }
            };
            println!("{}: ok ({summary})", file.display());
        }
        Command::Layout => {
            println!("workspace layout (schemaVersion {SCHEMA_VERSION})");
            println!("  {}", layout::MANIFEST);
            println!("  {}", layout::EXTENSIONS);
            println!("  {}/<source-id>.json", layout::SNAPSHOTS_DIR);
            println!(
                "  {}/<source-id>/<timestamp>_<origin>_<rand>.json",
                layout::INBOX_DIR
            );
        }
    }
    Ok(())
}
