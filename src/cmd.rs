use crate::{cookies::CookiesArgs, dl::DlArgs, sync::SyncArgs};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
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
    Dl(DlArgs),
    Sync(SyncArgs),
    Cookies(CookiesArgs),
}
