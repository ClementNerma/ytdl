use std::{fs, path::Path, time::Duration};

use anyhow::{bail, Context, Result};
use colored::Colorize;
use inquire::Confirm;

use crate::{
    config::{Config, PlatformConfig},
    cookies::existing_cookie_path,
    dl::{download, DlArgs},
    error, error_anyhow, info, info_inline,
    platforms::{build_platform_matchers, PlatformsMatchers},
    success,
    sync::{build_or_update_cache, display_sync, get_cache_path},
    warn,
};

use super::{cache::CacheEntry, SyncArgs};

pub fn sync_dl(args: &SyncArgs, config: &Config, sync_dir: &Path) -> Result<()> {
    let cache_path = get_cache_path(sync_dir, config);

    let cache = build_or_update_cache(sync_dir, config, &cache_path)?;

    display_sync(&cache);

    if args.dry_run {
        info!("Dry run completed!");
        return Ok(());
    }

    info!("");
    info!(
        "Going to download {} videos.",
        cache.entries.len().to_string().bright_yellow()
    );

    if cache.entries.len() != cache.max_index {
        info!(
            "{}",
            format!(
                "Found {} already downloaded videos.",
                cache.max_index - cache.entries.len()
            )
            .bright_black()
        );
    }

    if cache.entries.is_empty() {
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

    let entries = cache
        .entries
        .into_iter()
        .map(|entry| -> Result<(CacheEntry, &PlatformConfig)> {
            let platform = config.platforms.get(&entry.ie_key).with_context(|| {
                format!(
                    "Found unregistered IE key '{}' in videos list (video title: {})",
                    entry.ie_key.bright_yellow(),
                    entry.title.bright_cyan()
                )
            })?;

            Ok((entry, platform))
        })
        .collect::<Result<Vec<_>>>()?;

    let counter_len = entries.len().to_string().len();

    let mut failed = 0;

    let mut retrying = false;
    let mut i = 0;

    loop {
        i += 1;

        if i > entries.len() {
            break;
        }

        let (entry, platform) = entries.get(i - 1).unwrap();

        info!(
            "| Downloading video {:>width$} / {}: {}...",
            (i/* + 1 */).to_string().bright_yellow(),
            entries.len().to_string().bright_yellow(),
            entry.title.bright_magenta(),
            width = counter_len
        );

        if let Err(err) = sync_single(entry, platform, &matchers, config) {
            error_anyhow!(err);

            if retrying {
                error!("Failed twice on this item, skipping it.");
                failed += 1;
                continue;
            }

            warn!("");
            warn!("Failed on this video, waiting 5 seconds before retrying...");
            retrying = true;

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
    platform: &PlatformConfig,
    platforms_matchers: &PlatformsMatchers,
    config: &Config,
) -> Result<()> {
    let cookie_profile = platform
        .cookie_profile
        .as_ref()
        .map(|name| {
            existing_cookie_path(name, config)
                .map(|path| (name, path))
                .with_context(|| {
                    format!(
                        "Cookie profile '{}' was not found for platform '{}'",
                        name.bright_yellow(),
                        entry.ie_key.bright_cyan()
                    )
                })
        })
        .transpose()?;

    info!(
        "| Video from {} at {}{}",
        entry.ie_key.bright_cyan(),
        entry.url.bright_green(),
        match cookie_profile {
            Some((name, _)) => format!(" (with cookie profile {})", name.bright_yellow()),
            None => String::new(),
        }
    );

    download(
        &DlArgs {
            url: entry.url.clone(),
            format: None,
            custom_tmp_dir: None,
            output_dir: Some(entry.sync_dir.clone()),
            filenaming: None,
            limit_bandwidth: platform.bandwidth_limit.clone(),
            cookie_profile: cookie_profile
                .map(|(_, path)| match path.to_str() {
                    Some(path) => Ok(path.to_string()),
                    None => bail!(
                        "Cookie file path contains invalid UTF-8 characters: {}",
                        path.to_string_lossy().bright_magenta()
                    ),
                })
                .transpose()?,
            skip_repair_date: platform.skip_repair_date.unwrap_or(false),
            no_thumbnail: false,
            forward: vec![],
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
