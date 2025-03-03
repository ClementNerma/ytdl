mod cmd;
mod constants;
mod repair_date;

pub use cmd::*;
pub use constants::*;

use crate::{
    config::{Config, PlatformDownloadOptions, UseCookiesFrom},
    dl::repair_date::{apply_mtime, repair_date},
    error, error_anyhow, info, info_inline, success,
    utils::{
        platforms::{
            find_platform, try_find_platform, FoundPlatform, PlatformsMatchers,
            ID_REGEX_MATCHING_GROUP_NAME,
        },
        shell::run_cmd_bi_outs,
        ytdlp::{append_cookies_args, fetch_playlist},
    },
    warn,
};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::{
    env, fs,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub fn download(
    args: DlArgs,
    config: &Config,
    platform_matchers: &PlatformsMatchers,
) -> Result<()> {
    download_inner(args, config, platform_matchers)
}

fn download_inner(
    args: DlArgs,
    config: &Config,
    platform_matchers: &PlatformsMatchers,
) -> Result<()> {
    if args.no_platform && !args.skip_repair_date {
        bail!("Cannot repair date without a platform\n\n{REPAIR_DATE_EXPLANATION}");
    }

    let mut videos = Vec::with_capacity(args.urls.len());

    for url in &args.urls {
        let platform = try_find_platform(url, config, platform_matchers)?;

        if let Some(platform) = &platform {
            if platform.is_playlist {
                if args.urls.len() > 1 {
                    bail!("Cannot mix playlist and non-playlist downloads");
                }

                return download_playlist_inner(url, &args, config, platform, platform_matchers);
            }
        }

        videos.push((url, platform));
    }

    let colored_total = videos.len().to_string().bright_yellow();

    let mut failed = 0;

    for (i, (url, platform)) in videos.iter().enumerate() {
        let in_playlist = if videos.len() > 1 {
            if i > 0 {
                info!("");
            }

            info!(
                "> Downloading video {} / {colored_total}...",
                (i + 1).to_string().bright_yellow()
            );

            Some(PositionInPlaylist {
                index: i,
                total: videos.len(),
            })
        } else {
            None
        };

        let one_try = || {
            download_single_inner(url, *platform, &args, config, in_playlist)
                .inspect_err(|err| error_anyhow!(err))
        };

        if one_try().is_err() {
            warn!("\nFailed on this video, waiting 5 seconds before retrying...");

            std::thread::sleep(Duration::from_secs(5));

            warn!("\n> Retrying...\n");

            if one_try().is_err() {
                error!("\\!/ Failed twice on this item, skipping it. \\!/\n");
                failed += 1;
                continue;
            }
        }
    }

    if failed > 0 {
        bail!("Failed with {} errors", failed.to_string().bright_yellow());
    }

    Ok(())
}

// NOTE: ignores `args.url`
fn download_single_inner(
    url: &str,
    platform: Option<FoundPlatform>,
    args: &DlArgs,
    config: &Config,
    in_playlist: Option<PositionInPlaylist>,
) -> Result<()> {
    let video_id = platform
        .as_ref()
        .map(|platform| -> Result<String> {
            Ok(platform
                .platform_matchers
                .id_from_video_url
                .captures(url)
                .context("Failed to extract video ID from URL using the platform's matcher")?
                .name(ID_REGEX_MATCHING_GROUP_NAME)
                .unwrap()
                .as_str()
                .to_string())
        })
        .transpose()?;

    let dl_options =
        platform
            .map(|p| &p.platform_config.dl_options)
            .unwrap_or(&PlatformDownloadOptions {
                bandwidth_limit: None,
                needs_checking: None,
                rate_limited: None,
                cookies: None,
                skip_repair_date: Some(true),
                output_format: None,
                download_format: None,
                no_thumbnail: None,
                forward_ytdlp_args: None,
            });

    let mut ytdl_args = vec![
        "--format",
        args.format
            .as_deref()
            .or(dl_options.download_format.as_deref())
            .unwrap_or(DEFAULT_BEST_VIDEO_FORMAT),
        "--add-metadata",
        "--abort-on-unavailable-fragment",
        "--compat-options",
        "abort-on-error",
    ];

    let bandwidth_limit = args
        .limit_bandwidth
        .as_ref()
        .or(dl_options.bandwidth_limit.as_ref())
        .or(config.default_bandwidth_limit.as_ref());

    if let Some(bandwidth_limit) = bandwidth_limit {
        ytdl_args.push("--limit-rate");
        ytdl_args.push(bandwidth_limit);
    };

    let cwd = env::current_dir().context("Failed to get current directory")?;

    let mut output_dir = args.output_dir.clone().unwrap_or_else(|| cwd.clone());

    if output_dir.ends_with("/") {
        output_dir.pop();
    }

    if !output_dir.is_dir() {
        bail!(
            "Output directory does not exist at path: {}",
            output_dir.to_string_lossy().bright_magenta()
        );
    }

    let output_dir_display = if output_dir == Path::new(".") || output_dir == cwd {
        format!(
            ". ({})",
            cwd.file_name().unwrap().to_string_lossy().bright_cyan()
        )
    } else {
        output_dir.to_string_lossy().to_string()
    };

    let tmp_dir = if args.no_temp_dir {
        None
    } else {
        let tmp_dir = args.custom_temp_dir.as_ref().unwrap_or(&config.tmp_dir);

        if !tmp_dir.is_dir() {
            fs::create_dir(tmp_dir).with_context(|| {
                format!(
                    "Failed to create temporary directory at path: {}",
                    tmp_dir.to_string_lossy().bright_magenta()
                )
            })?;
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        Some(tmp_dir.join(format!("{}-{}", now.as_secs(), now.subsec_micros())))
    };

    if tmp_dir.is_none() && !dl_options.skip_repair_date.unwrap_or(false) && !args.skip_repair_date
    {
        bail!("Cannot repair date in a non-temporary directory.\n\n{REPAIR_DATE_EXPLANATION}");
    }

    if !args.no_thumbnail && dl_options.no_thumbnail != Some(true) {
        ytdl_args.push("--embed-thumbnail");

        if let Some(format) = &dl_options.output_format {
            ytdl_args.push("--merge-output-format");
            ytdl_args.push(format);
        }
    }

    let cookies = args.cookies.as_ref().or(dl_options.cookies.as_ref());

    if let Some(cookies) = cookies {
        append_cookies_args(&mut ytdl_args, cookies)?;
    }

    if dl_options.rate_limited == Some(true) || args.rate_limited {
        ytdl_args.push("--sleep-requests=3");
    }

    let filenaming = args.filenaming.as_deref().unwrap_or(DEFAULT_FILENAMING);

    let dl_dir = match &tmp_dir {
        Some(tmp_dir) => tmp_dir.clone(),
        None => output_dir.clone(),
    };

    let output_with_filenaming = dl_dir.join(if args.index_prefix {
        let in_playlist = in_playlist.context(
            "Cannot add an index prefix as this video isn't part of a playlist download",
        )?;

        format!(
            "{:0total_len$}. {filenaming}",
            in_playlist.index + 1,
            total_len = in_playlist.total.to_string().len()
        )
    } else {
        filenaming.to_owned()
    });

    ytdl_args.push("-o");
    ytdl_args.push(
        output_with_filenaming
            .to_str()
            .context("Output directory contains invalid UTF-8 characters")?,
    );

    ytdl_args.push(url);

    info!(
        "> Downloading video {}{}",
        match &platform {
            Some(platform) => format!("from platform {}", platform.platform_name.bright_cyan()),
            None => "without a platform".bright_yellow().to_string(),
        },
        match cookies {
            Some(UseCookiesFrom::Browser(name)) =>
                format!(" (with cookies from browser {})", name.bright_yellow()),
            Some(UseCookiesFrom::File(path)) =>
                format!(" (with cookies from file {})", path.bright_magenta()),
            None => String::new(),
        }
    );

    if let Some(args) = &dl_options.forward_ytdlp_args {
        info!(
            "| Forwarding additional YT-DLP arguments from platform configuration: {}",
            args.join(" ").bright_yellow()
        );

        ytdl_args.extend(args.iter().map(String::as_str));
    }

    if !args.forward_ytdlp_args.is_empty() {
        info!(
            "| Forwarding additional YT-DLP arguments from command line: {}",
            args.forward_ytdlp_args.join(" ").bright_yellow()
        );
        ytdl_args.extend(args.forward_ytdlp_args.iter().map(String::as_str));
    }

    if let Some(tmp_dir) = &tmp_dir {
        info!(
            "> Downloading first to temporary directory: {}",
            tmp_dir.to_string_lossy().bright_magenta()
        );
        info!(
            "> Then moving to provided final directory: {}",
            output_dir_display.bright_magenta()
        );
    }

    // Actually calling YT-DLP here
    run_cmd_bi_outs(&config.yt_dlp_bin, &ytdl_args, Some(&inspect_err))
        .context("Failed to run YT-DLP")?;

    if tmp_dir.is_none() {
        return Ok(());
    }

    let mut files =
        fs::read_dir(dl_dir.clone()).context("Failed to read the temporary download directory")?;

    let video_file = files
        .next()
        .context("No file found in the temporary download directory")?
        .context("Failed to get informations on the downloaded video file")?
        .path();

    if files.next().is_some() {
        bail!("Found more than one video in the temporary download directory");
    }

    assert!(
        video_file.is_file(),
        "Found non-file item in the temporary download directory: {}",
        video_file.display()
    );

    let repair_dates = if !args.skip_repair_date && dl_options.skip_repair_date != Some(true) {
        info!("> Repairing date as requested");

        repair_date(
            &video_file,
            &video_id.unwrap(),
            &config.yt_dlp_bin,
            platform.unwrap().platform_config,
            cookies,
        )?
    } else {
        None
    };

    info!(
        "> Moving the download file to output directory: {}...",
        output_dir.to_string_lossy().bright_magenta()
    );

    let output_file = output_dir.join(video_file.file_name().unwrap());

    fs::copy(&video_file, &output_file).with_context(|| {
        format!(
            "Failed to move downloaded file: {}",
            video_file.to_string_lossy().bright_magenta()
        )
    })?;

    fs::remove_file(&video_file).with_context(|| format!("Failed to remove temporary download file at path: {}, directory will not be cleaned up",
        video_file.to_string_lossy().bright_magenta()
    ))?;

    if let Some(date) = repair_dates {
        info!("> Applying repaired date...");

        apply_mtime(&output_file, date).with_context(|| {
            format!(
                "Failed to apply modification time for file '{}'",
                output_file.display()
            )
        })?;

        success!("> Successfully repaired dates!");
    }

    fs::remove_dir(&dl_dir).with_context(|| {
        format!(
            "Failed to remove temporary directory at path: {}",
            dl_dir.to_string_lossy().bright_magenta()
        )
    })?;

    success!("> Done!");

    Ok(())
}

fn download_playlist_inner(
    playlist_url: &str,
    args: &DlArgs,
    config: &Config,
    platform: &FoundPlatform,
    platform_matchers: &PlatformsMatchers,
) -> Result<()> {
    let FoundPlatform {
        platform_name,
        platform_config,
        is_playlist,
        platform_matchers: _,
    } = platform;

    assert!(is_playlist);

    info!("Fetching playlist's content...");

    let playlist = fetch_playlist(&config.yt_dlp_bin, playlist_url, args.cookies.as_ref())
        .context("Failed to fetch the playlist's content")?;

    let colored_total = playlist.entries.len().to_string().bright_yellow();

    info!("Detected {} videos.", colored_total);
    info!("");

    let mut urls = Vec::with_capacity(playlist.entries.len());

    for video in &playlist.entries {
        let platform = find_platform(&video.url, config, platform_matchers)?;

        let url = if platform.platform_name == *platform_name {
            video.url.clone()
        } else if platform_config.redirect_playlist_videos == Some(true) {
            let video_id = platform.platform_matchers
                .id_from_video_url
                .captures(&video.url)
                .with_context(|| {
                    format!(
                        "Failed to extract video ID from URL ({}) using the platform's ({}) matcher",
                        video.url.bright_magenta(),
                        platform.platform_name.bright_cyan()
                    )
                })?
                .name(ID_REGEX_MATCHING_GROUP_NAME)
                .unwrap()
                .as_str()
                .to_string();

            format!("{}{}", platform_config.videos_url_prefix, video_id)
        } else {
            warn!("Skipping video {} as it belongs to another platform. Use --redirect-playlist-videos to download anyway.", video.url.bright_magenta());
            continue;
        };

        urls.push(url);
    }

    download_inner(
        DlArgs {
            urls,
            cookies: platform_config
                .dl_options
                .cookies
                .clone()
                .or(args.cookies.clone()),
            ..args.clone()
        },
        config,
        platform_matchers,
    )
}

fn inspect_err(err: &str) {
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

#[derive(Clone, Copy)]
struct PositionInPlaylist {
    index: usize,
    total: usize,
}

static REPAIR_DATE_EXPLANATION: &str = r#"
By default, ytdl tries to write the videos' upload date to the downloaded files' metadata.

This requires specific support by the platform by the platform you're downloading from,
and also to use a temporary directory.

If you wish to disable this behaviour, use the `--skip-repair-date` option, or configure it
in your ytdl-config.json file.
"#;
