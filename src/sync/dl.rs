use std::{fs, path::Path, time::Duration};

use anyhow::{bail, Context, Result};
use colored::Colorize;
use inquire::Confirm;

use crate::{
    config::Config,
    dl::{download, DlArgs},
    error, error_anyhow, info, info_inline,
    platforms::{build_platform_matchers, PlatformsMatchers},
    success,
    sync::{build_or_update_cache, display_sync, get_cache_path},
    warn,
};

use super::{cache::CacheEntry, SyncArgs};

pub fn sync_dl(args: SyncArgs, config: &Config, sync_dir: &Path) -> Result<()> {
    if let Some(url) = &args.url {
        let sync_file = sync_dir.join(&config.url_filename);

        if sync_file.exists() {
            let existing_url = fs::read_to_string(&sync_file)
                .context("Failed to read the synchronization file")?;

            if existing_url == *url {
                warn!(
                    "Provided URL is already specified in the synchronization file, doing nothing."
                );
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
            fs::write(&sync_file, url).context("Failed to create the synchronization file")?;
        }
    }

    let cache_path = get_cache_path(sync_dir, config);

    let cache = build_or_update_cache(sync_dir, config, &cache_path)?;

    display_sync(&cache);

    if args.dry_run {
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

    let matchers = build_platform_matchers(config)?;

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

    let mut retrying = None;

    for (i, entry) in entries.iter().enumerate() {
        info!(
            "| Downloading video {:>width$} / {}: {}...",
            (i + 1).to_string().bright_yellow(),
            entries.len().to_string().bright_yellow(),
            entry.title.bright_magenta(),
            width = counter_len
        );

        if let Err(err) = sync_single(entry, &matchers, config) {
            error_anyhow!(err);

            if retrying == Some(i) {
                error!("Failed twice on this item, skipping it.");
                failed += 1;
                continue;
            }

            warn!("");
            warn!("Failed on this video, waiting 5 seconds before retrying...");
            retrying = Some(i);

            std::thread::sleep(Duration::from_secs(5));

            continue;
        }

        info!("");
    }

    if failed > 0 {
        bail!("Failed with {} errors", failed.to_string().bright_yellow());
    }

    fs::remove_file(&cache_path).context("Failed to remove the cache file")?;

    Ok(())
}

fn sync_single(
    entry: &CacheEntry,
    platforms_matchers: &PlatformsMatchers,
    config: &Config,
) -> Result<()> {
    info!(
        "| Video from {} at {}",
        entry.ie_key.bright_cyan(),
        entry.url.bright_green(),
    );

    download(
        DlArgs {
            url: entry.url.clone(),
            output_dir: Some(entry.sync_dir.clone()),
            ..Default::default()
        },
        config,
        platforms_matchers,
        Some(&wait_sync_when_too_many_requests),
    )
}

fn wait_sync_when_too_many_requests(err: &str) {
    if !err.contains("HTTP Error 429: Too Many Requests.") {
        return;
    }

    warn!("Failed due to too many requests being made to server.");

    let mut remaining = 15 * 60;

    while remaining > 0 {
        let remaining_msg = format!(
            "{}{}s",
            if remaining > 60 {
                format!("{}m ", remaining / 60)
            } else {
                String::new()
            },
            remaining % 60
        )
        .bright_cyan();

        let message = format!(">> Waiting before retry... {}", remaining_msg).bright_yellow();

        info_inline!("\r{}", message);
        std::thread::sleep(Duration::from_secs(1));
        remaining -= 1;
    }
}
