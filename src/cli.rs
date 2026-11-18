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
    Validate(ValidateCommand),
    Import(ImportCommand),
    ImportWordpress(ImportWordpressCommand),
    Serve(ServeCommand),
}

#[derive(Debug, Clone, Args)]
pub struct PostSourceArguments {
    #[arg(long, default_value = "src")]
    pub posts_directory: PathBuf,
    #[arg(long)]
    pub media_directory: Option<PathBuf>,
    #[arg(long, default_value = "ru")]
    pub default_language: String,
}

#[derive(Debug, Args)]
pub struct ValidateCommand {
    #[command(flatten)]
    pub source: PostSourceArguments,
    #[arg(long)]
    pub strict: bool,
}

#[derive(Debug, Args)]
pub struct ImportCommand {
    #[command(flatten)]
    pub source: PostSourceArguments,
    #[arg(long, default_value = "blog.redb")]
    pub database: PathBuf,
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "ru,en,zh,de,ja,fr,es,pt,hi"
    )]
    pub languages: Vec<String>,
    #[arg(long, default_value_t = 10)]
    pub posts_per_page: usize,
    #[arg(long, default_value = "Zefir1990")]
    pub site_title: String,
    #[arg(long)]
    pub strict: bool,
}

#[derive(Debug, Args)]
pub struct ImportWordpressCommand {
    #[arg(long)]
    pub slug: String,
    #[arg(long, default_value = "blog.redb")]
    pub database: PathBuf,
    #[arg(long, default_value = "https://demensdeum.com/blog")]
    pub base_url: String,
    #[arg(long, default_value = "en")]
    pub wordpress_default_language: String,
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "en,ru,zh,de,ja,fr,es,pt,hi"
    )]
    pub languages: Vec<String>,
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
