#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod cmd;
mod config;
mod dl;
mod logging;
mod platforms;
mod shell;
mod sync;
mod ytdlp;

use self::{
    cmd::{Action, Cmd},
    config::Config,
    ytdlp::check_version,
};
use anyhow::{bail, Context, Result};
use clap::Parser;
use dirs::config_dir;
use dl::download;
use std::{env, fs};
use sync::sync_dl;

fn main() {
    if let Err(err) = inner_main() {
        error_anyhow!(err);
        std::process::exit(1);
    }
}

fn inner_main() -> Result<()> {
    let args = Cmd::parse();

    let default_config_path = config_dir()
        .context("Failed to determine path to the configuration directory")?
        .join("ytdl")
        .join("config.json");

    let config_path = args.config_file.unwrap_or(default_config_path);

    if !config_path.is_file() {
        bail!("Config file was not found at: {}", config_path.display());
    }

    let config = fs::read_to_string(&config_path)
        .unwrap_or_else(|e| format!("Failed to read config file: {e}"));

    let config = Config::decode(&config)?;

    if let Err(e) = check_version(&config.yt_dlp_bin) {
        bail!("Failed to check YT-DLP: {e}");
    }

    let cwd = env::current_dir().context("Failed to get current directory")?;

    match args.action {
        Action::Dl(args) => download(&args, &config, None, None),
        Action::Sync(args) => sync_dl(&args, &config, &cwd),
    }
}
