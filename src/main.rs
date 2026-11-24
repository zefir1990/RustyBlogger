mod assets;
mod cli;
mod date;
mod model;
mod render;
mod storage;
mod templates;
mod web;

use anyhow::{bail, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Command, ServeCommand};
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

async fn run_serve(command: ServeCommand) -> Result<i32> {
    if !command.database.is_file() {
        bail!("database {} does not exist", command.database.display());
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
