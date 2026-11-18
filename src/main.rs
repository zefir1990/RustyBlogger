mod cli;
mod importer;
mod model;
mod parser;
mod render;
mod storage;
mod templates;
mod web;

use anyhow::{bail, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Command, ImportCommand, ServeCommand, ValidateCommand};
use crate::importer::{build_snapshot, SnapshotOptions};
use crate::storage::RedbPostStore;
use crate::web::{serve, ServeOptions};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Validate(command) => run_validate(command),
        Command::Import(command) => run_import(command),
        Command::Serve(command) => run_serve(command).await,
    };
    match result {
        Ok(0) => {}
        Ok(_) => std::process::exit(1),
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::exit(1);
        }
    }
}

fn run_validate(command: ValidateCommand) -> Result<i32> {
    let snapshot_options = SnapshotOptions {
        posts_directory: &command.source.posts_directory,
        media_directory: command.source.media_directory.as_deref(),
        default_language: &command.source.default_language,
        configured_languages: Vec::new(),
        posts_per_page: 10,
    };
    let (_, report) = build_snapshot(&snapshot_options)?;
    report.print();
    if report.has_errors() || (command.strict && report.has_warnings()) {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn run_import(command: ImportCommand) -> Result<i32> {
    let snapshot_options = SnapshotOptions {
        posts_directory: &command.source.posts_directory,
        media_directory: command.source.media_directory.as_deref(),
        default_language: &command.source.default_language,
        configured_languages: command.languages,
        posts_per_page: command.posts_per_page,
    };
    let (snapshot, report) = build_snapshot(&snapshot_options)?;
    report.print();
    if report.has_errors() {
        eprintln!("import aborted because the posts contain syntax errors");
        return Ok(1);
    }
    if command.strict && report.has_warnings() {
        eprintln!("import aborted in strict mode because the posts contain warnings");
        return Ok(1);
    }
    if snapshot.posts.is_empty() {
        eprintln!("import aborted because no valid posts were found");
        return Ok(1);
    }
    let store = RedbPostStore::open(&command.database)?;
    store.replace_snapshot(&snapshot)?;
    println!(
        "imported {} posts in {} languages into {}",
        snapshot.posts.len(),
        snapshot.languages.len(),
        command.database.display()
    );
    Ok(0)
}

async fn run_serve(command: ServeCommand) -> Result<i32> {
    if !command.database.is_file() {
        bail!(
            "database {} does not exist, run the import command first",
            command.database.display()
        );
    }
    if !command.media_directory.is_dir() {
        tracing::warn!(
            "media directory {} does not exist, media requests will return 404",
            command.media_directory.display()
        );
    }
    serve(ServeOptions {
        database_path: command.database,
        media_directory: command.media_directory,
        bind_address: command.bind_address,
        site_title: command.site_title,
    })
    .await?;
    Ok(0)
}
