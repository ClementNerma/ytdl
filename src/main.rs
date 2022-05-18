#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

use colored::Colorize;

mod blacklist;
mod builder;
mod cache;
mod cmd;
mod config;
mod logging;
mod ytdlp;

fn main() {
    use crate::{cmd::Args, config::Config, ytdlp::check_version};
    use clap::Parser;

    let args = Args::parse();

    let config = Config::decode(&args.config).unwrap_or_else(|e| fail!("{e}"));

    if let Err(e) = check_version() {
        fail!("Failed to check YT-DLP: {e}");
    }

    let cache =
        builder::build_or_update_cache(&args.sync_dir, &config).unwrap_or_else(|e| fail!("{e}"));

    if args.display_colored_list {
        let max_index = cache
            .entries
            .iter()
            .map(|entry| entry.index)
            .max()
            .unwrap_or(0);

        let str_len = max_index.to_string().len();

        for entry in cache.entries {
            let counter_str =
                format!("{:>str_len$} / {}", entry.index + 1, cache.max_index).bright_black();

            let sync_dir = entry.sync_dir.to_string_lossy();

            info!(
                "{} {} {}{}",
                counter_str,
                format!("[{}]", entry.id).bright_magenta(),
                if sync_dir == "." {
                    String::new()
                } else {
                    format!(" {}", sync_dir.bright_cyan())
                },
                entry.title.bright_yellow()
            );
        }
    }
}
