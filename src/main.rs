#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod cmd;
mod config;
mod cookies;
mod dl;
mod sync;
mod utils;

use colored::Colorize;
use cookies::cookies;
use utils::platforms::build_platform_matchers;
pub use utils::*;

use self::{
    cmd::{Action, Cmd},
    config::Config,
    utils::ytdlp::check_version,
};
use anyhow::{bail, Context, Result};
use clap::Parser;
use dirs::config_dir;
use dl::download;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};
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

    let mut config: Config =
        serde_json::from_str(&config).context("Failed to decode provided configuration")?;

    if !config.profiles_dir.is_absolute() {
        config.profiles_dir = config_path.parent().unwrap().join(&config.profiles_dir);
    }

    if !config.profiles_dir.is_dir() {
        bail!(
            "Cookies directory was not found at: {}",
            config.profiles_dir.display()
        );
    }

    if let Err(e) = check_version(&config.yt_dlp_bin) {
        bail!("Failed to check YT-DLP: {e}");
    }

    let cwd = env::current_dir().context("Failed to get current directory")?;

    match args.action {
        Action::Dl(args) => download(args, &config, &build_platform_matchers(&config)?, None),
        Action::Sync(args) => sync_dl(args, &config, &cwd),
        Action::Cookies(args) => cookies(args, &config),
        Action::InitConfig => Ok(()),
    }
}

fn create_config_file(config_file_path: &Path) -> Result<()> {
    info!(
        "Initializing a configuration file at: {}",
        config_file_path.display()
    );

    let profiles_dir_name = "profiles";

    let cfg = Config {
        yt_dlp_bin: PathBuf::from("yt-dlp"),
        profiles_dir: PathBuf::from(profiles_dir_name),
        tmp_dir: PathBuf::from("/tmp/ytdl"),
        url_filename: ".ytdlsync-url".to_string(),
        cache_filename: ".ytdlsync-cache".to_string(),
        auto_blacklist_filename: ".ytdlsync-blacklist".to_string(),
        custom_blacklist_filename: ".ytdlsync-custom-blacklist".to_string(),
        default_bandwidth_limit: None,
        platforms: HashMap::new(),
    };

    let par = config_file_path.parent().unwrap();

    if !par.exists() {
        fs::create_dir_all(par)
            .context("failed to create parent directories for configuration file")?;
    }

    let profiles_dir = par.join(profiles_dir_name);

    if !profiles_dir.exists() {
        fs::create_dir_all(&profiles_dir)
            .context("failed to create the profiles directories for configuration file")?;
    }

    fs::write(
        config_file_path,
        serde_json::to_string_pretty(&cfg).unwrap(),
    )
    .context("failed to write default configuration file")?;

    if !cfg.tmp_dir.exists() {
        fs::create_dir_all(&cfg.tmp_dir)
            .context("failed to create the temporary downloads directory")?;
    }

    success!("Default configuration file was successfully created!");

    Ok(())
}
