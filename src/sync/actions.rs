use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use colored::Colorize;
use inquire::Confirm;

use crate::{
    config::Config,
    dl::{download, DlArgs},
    info, success,
    sync::{blacklist::BlacklistEntry, builder::get_cache_path},
    utils::platforms::build_platform_matchers,
    warn,
};

use super::{
    blacklist::blacklist_video, builder::build_or_update_cache, cmd::SyncAction,
    display::display_sync, SyncArgs,
};

pub fn sync(args: SyncArgs, config: &Config, sync_dir: &Path) -> Result<()> {
    let SyncArgs { action } = args;

    match action {
        SyncAction::Setup { url } => setup(&url, config, sync_dir),
        SyncAction::Run { dry_run } => run(dry_run, config, sync_dir),
        SyncAction::Blacklist { platform, video_id } => {
            blacklist(BlacklistEntry::new(platform, video_id), config, sync_dir)
        }
    }
}

fn setup(url: &str, config: &Config, sync_dir: &Path) -> Result<()> {
    let sync_file = sync_dir.join(&config.url_filename);

    if sync_file.exists() {
        let existing_url =
            fs::read_to_string(&sync_file).context("Failed to read the synchronization file")?;

        if existing_url == *url {
            warn!("Provided URL is already specified in the synchronization file, doing nothing.");
            Ok(())
        } else {
            bail!(
                "This directory already has a synchronization file with a different URL ({})",
                existing_url.bright_cyan()
            );
        }
    } else {
        info!(
            "Creating a synchronization file for URL: {}",
            url.bright_cyan()
        );

        fs::write(&sync_file, url).context("Failed to create the synchronization file")
    }
}

fn blacklist(entry: BlacklistEntry, config: &Config, sync_dir: &Path) -> Result<()> {
    if !config.platforms.contains_key(entry.ie_key()) {
        bail!(
            "Unkonwn IE key '{}'. Registered platforms are: {}",
            entry.ie_key(),
            config
                .platforms
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
                .bright_cyan()
        );
    }

    blacklist_video(&sync_dir.join(&config.custom_blacklist_filename), &entry)
}

fn run(dry_run: bool, config: &Config, sync_dir: &Path) -> Result<()> {
    let cache_path = get_cache_path(sync_dir, config);

    let cache = build_or_update_cache(sync_dir, config, &cache_path)?;

    display_sync(&cache);

    if dry_run {
        info!("Dry run completed!");
        return Ok(());
    }

    let entries = cache.entries;

    info!("");
    info!(
        "Going to download {} videos.",
        entries.len().to_string().bright_yellow()
    );

    if entries.len() != cache.max_index {
        info!(
            "{}",
            format!(
                "Found {} already downloaded videos.",
                cache.max_index - entries.len()
            )
            .bright_black()
        );
    }

    if entries.is_empty() {
        success!("Nothing to download!");
        fs::remove_file(&cache_path)?;
        return Ok(());
    }

    let platform_matchers = build_platform_matchers(config)?;

    info!("");
    info!("Do you want to continue?");

    let ans = Confirm::new("Please confirm")
        .with_default(true)
        .prompt()
        .context("Failed to setup or retrieve confirmation prompt")?;

    info!("");

    if !ans {
        warn!("Aborting synchronization.");
        return Ok(());
    }

    for entry in &entries {
        if !config.platforms.contains_key(&entry.ie_key) {
            bail!(
                "Found unregistered IE key '{}' in videos list (video title: {})",
                entry.ie_key.bright_yellow(),
                entry.title.bright_cyan()
            );
        }
    }

    let counter_len = entries.len().to_string().len();

    let mut failed = 0;

    for (i, entry) in entries.iter().enumerate() {
        info!(
            "| Downloading video {:>width$} / {}: {}...",
            (i + 1).to_string().bright_yellow(),
            entries.len().to_string().bright_yellow(),
            entry.title.bright_magenta(),
            width = counter_len
        );

        let result = download(
            DlArgs {
                urls: vec![entry.url.clone()],
                output_dir: Some(entry.sync_dir.clone()),
                ..Default::default()
            },
            config,
            &platform_matchers,
        );

        if result.is_err() {
            failed += 1;
        }
    }

    if failed > 1 {
        bail!("Failed with {failed} errors");
    }

    fs::remove_file(&cache_path).context("Failed to remove the cache file")?;

    Ok(())
}
