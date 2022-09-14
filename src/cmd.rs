use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cmd {
    #[clap(
        short = 'c',
        long = "config-file",
        help = "Path to the configuration file"
    )]
    pub config_file: Option<PathBuf>,

    #[clap(subcommand)]
    pub action: Action,
}

#[derive(Subcommand)]
pub enum Action {
    Sync(SyncArgs),
}

#[derive(Args)]
pub struct SyncArgs {
    #[clap(long = "dry-run", help = "Simulate the synchronization")]
    pub dry_run: bool,
}
