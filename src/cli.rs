use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "rustyblogger",
    version,
    about = "Multilingual blog engine backed by the redb embedded database"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Serve(ServeCommand),
}

#[derive(Debug, Args)]
pub struct ServeCommand {
    #[arg(long, default_value = "blog.redb")]
    pub database: PathBuf,
    #[arg(long, default_value = "media")]
    pub media_directory: PathBuf,
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub bind_address: String,
    #[arg(long)]
    pub site_title: Option<String>,
}
