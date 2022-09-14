#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod cmd;
mod config;
mod logging;
mod sync;
mod ytdlp;

use self::{
    cmd::{Action, Cmd},
    config::Config,
    sync::{build_or_update_cache, display_sync},
    ytdlp::check_version,
};
use clap::Parser;
use dirs::config_dir;
use std::{env, fs};

fn main() {
    let args = Cmd::parse();
    let default_config_path = config_dir().unwrap().join("yt-dlp").join("config.json");
    let config_path = args.config_file.unwrap_or(default_config_path);

    if !config_path.is_file() {
        fail!("Config file was not found at: {}", config_path.display());
    }

    let config = fs::read_to_string(&config_path)
        .unwrap_or_else(|e| format!("Failed to read config file: {e}"));

    let config = Config::decode(&config).unwrap_or_else(|e| fail!("{e}"));

    if let Err(e) = check_version(&config.yt_dlp_bin) {
        fail!("Failed to check YT-DLP: {e}");
    }

    let cwd = env::current_dir().unwrap();

    match args.action {
        Action::Sync(args) => {
            let cache = build_or_update_cache(&cwd, &config).unwrap_or_else(|e| fail!("{e}"));

            display_sync(&cache);

            if args.dry_run {
                info!("Dry run completed!");
                return;
            }

            todo!()
        }
    }
}
