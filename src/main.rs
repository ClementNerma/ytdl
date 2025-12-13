#![forbid(unsafe_code)]
#![forbid(unused_must_use)]
#![warn(unused_crate_dependencies)]

mod cmd;
mod config;
mod dl;
mod sync;
mod utils;

use std::{env, fs, path::Path};

use anyhow::{bail, Context, Result};
use clap::Parser;
use colored::Colorize;
use dirs::config_dir;

use self::{
    cmd::{Action, Cmd},
    config::Config,
    dl::{album::download_album, download_from_args},
    sync::sync,
    utils::{platforms::build_platform_matchers, ytdlp::check_version},
};

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
        .join("ytdl-config.json");

    let config_path = args.config_file.unwrap_or(default_config_path);

    if matches!(args.action, Action::InitConfig) {
        if config_path.exists() {
            bail!(
                "A configuration file already exists at path {}",
                config_path.to_string_lossy().bright_magenta()
            );
        }

        create_config_file(&config_path)?;
    }

    if !config_path.is_file() {
        bail!("No configuration file found (you can create one with the 'init-config' subcommand)");
    }

    let config = fs::read_to_string(&config_path).context("Failed to read config file")?;

    let config = serde_json::from_str::<Config>(&config)
        .context("Failed to decode provided configuration")?;

    if !config.tmp_dir.exists() {
        fs::create_dir_all(&config.tmp_dir)
            .context("failed to create the temporary downloads directory")?;
    }

    if let Err(e) = check_version(&config.yt_dlp_bin) {
        bail!("Failed to check YT-DLP: {e}");
    }

    let cwd = env::current_dir().context("Failed to get current directory")?;

    match args.action {
        Action::Dl(args) => download_from_args(args, &config, &build_platform_matchers(&config)?),
        Action::Sync(args) => sync(args, &config, &cwd),
        Action::Album(args) => download_album(args, &config, &cwd),
        Action::InitConfig => Ok(()),
    }
}

fn create_config_file(config_file_path: &Path) -> Result<()> {
    info!(
        "Initializing a configuration file at: {}",
        config_file_path.display()
    );

    let cfg = Config::default();

    let parent = config_file_path.parent().unwrap();

    if !parent.exists() {
        fs::create_dir_all(parent)
            .context("failed to create parent directories for configuration file")?;
    }

    fs::write(
        config_file_path,
        serde_json::to_string_pretty(&cfg).unwrap(),
    )
    .context("failed to write default configuration file")?;

    success!("Default configuration file was successfully created!");

    Ok(())
}
