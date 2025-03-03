mod cmd;
mod constants;
mod repair_date;

pub use cmd::*;
pub use constants::*;
use pomsky_macro::pomsky;
use regex::Regex;

use crate::{
    config::{Config, PlatformDownloadOptions, UseCookiesFrom},
    dl::repair_date::{apply_mtime, parse_date},
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
    collections::HashMap,
    env, fs,
    path::Path,
    sync::LazyLock,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub fn download_from_args(
    args: DlArgs,
    config: &Config,
    platform_matchers: &PlatformsMatchers,
) -> Result<()> {
    let DlArgs { urls, dl_url } = args;

    let items = urls
        .into_iter()
        .map(|url| (url, dl_url.clone()))
        .collect::<Vec<_>>();

    download(&items, config, platform_matchers)
}

pub fn download(
    urls: &[(String, SingleDlArgs)],
    config: &Config,
    platform_matchers: &PlatformsMatchers,
) -> Result<()> {
    download_inner(urls, config, platform_matchers)
}

fn download_inner(
    urls: &[(String, SingleDlArgs)],
    config: &Config,
    platform_matchers: &PlatformsMatchers,
) -> Result<()> {
    for (_, args) in urls {
        if args.no_platform && !args.skip_repair_date {
            bail!("Cannot repair date without a platform\n\n{REPAIR_DATE_EXPLANATION}");
        }
    }

    let mut videos = Vec::with_capacity(urls.len());

    for (url, args) in urls {
        let platform = try_find_platform(url, config, platform_matchers)?;

        if let Some(platform) = &platform {
            if platform.is_playlist {
                if urls.len() > 1 {
                    bail!("Cannot mix playlist and non-playlist downloads");
                }

                return download_playlist_inner(url, args, config, platform, platform_matchers);
            }
        }

        videos.push((url, args, platform));
    }

    let colored_total = videos.len().to_string().bright_yellow();

    let mut failed = 0;

    let mut last_dl_from_platforms = HashMap::<&str, Instant>::new();

    for (i, (url, args, platform)) in videos.iter().enumerate() {
        let in_playlist = if videos.len() > 1 {
            if i > 0 {
                info!("");
            }

            info!(
                "> Downloading video {} / {colored_total}{}...",
                (i + 1).to_string().bright_yellow(),
                match &args.prefetched_title {
                    Some(title) => format!(": {}", title.bright_magenta()),
                    None => String::new(),
                }
            );

            Some(PositionInPlaylist {
                index: i,
                total: videos.len(),
            })
        } else {
            None
        };

        let rate_limited_platform_name = platform
            .filter(|p| p.platform_config.dl_options.rate_limited == Some(true))
            .map(|p| p.platform_name);

        if args.rate_limited {
            warn!(
                "| Rate limited download requested, waiting {} seconds before downloading...",
                RATE_LIMITED_WAIT_DURATION_SECS
            );
            std::thread::sleep(Duration::from_secs(RATE_LIMITED_WAIT_DURATION_SECS));
        } else if let Some(last_dl) =
            rate_limited_platform_name.and_then(|name| last_dl_from_platforms.get(name))
        {
            let mut remaining_wait = Duration::from_secs(RATE_LIMITED_WAIT_DURATION_SECS)
                .saturating_sub(last_dl.elapsed());

            if !remaining_wait.is_zero() {
                // Round up
                if remaining_wait.subsec_millis() > 0 {
                    remaining_wait += Duration::from_secs(1);
                }

                warn!("| Platform is rate limited!");
                warn!(
                    "| Waiting {} seconds before downloading from the same platform again...",
                    remaining_wait.as_secs()
                );

                std::thread::sleep(Duration::from_secs(remaining_wait.as_secs()));
            }
        }

        let one_try = || {
            download_single_inner(url, *platform, args, config, in_playlist)
                .inspect_err(|err| error_anyhow!(err))
        };

        if one_try().is_err() {
            let wait_duration = if rate_limited_platform_name.is_some() {
                RATE_LIMITED_WAIT_DURATION_SECS
            } else {
                AFTER_FAILURE_WAIT_DURATION_SECS
            };

            warn!("\nFailed on this video, waiting {wait_duration} seconds before retrying...");

            std::thread::sleep(Duration::from_secs(wait_duration));

            warn!("\n> Retrying...\n");

            if one_try().is_err() {
                error!("\\!/ Failed twice on this item, skipping it. \\!/\n");
                failed += 1;
            }
        }

        if let Some(platform_name) = rate_limited_platform_name {
            last_dl_from_platforms.insert(platform_name, Instant::now());
        }
    }

    if failed > 0 {
        bail!(
            "Failed with {} error(s)",
            failed.to_string().bright_yellow()
        );
    }

    Ok(())
}

fn download_single_inner(
    url: &str,
    platform: Option<FoundPlatform>,
    args: &SingleDlArgs,
    config: &Config,
    in_playlist: Option<PositionInPlaylist>,
) -> Result<()> {
    let platform_dl_options =
        platform
            .map(|p| &p.platform_config.dl_options)
            .unwrap_or(&PlatformDownloadOptions {
                bandwidth_limit: None,
                needs_checking: None,
                rate_limited: None,
                cookies: None,
                skip_repair_date: None,
                output_format: None,
                download_format: None,
                no_thumbnail: None,
                forward_ytdlp_args: None,
            });

    let mut ytdl_args = vec![
        "--format",
        args.format
            .as_deref()
            .or(platform_dl_options.download_format.as_deref())
            .unwrap_or(DEFAULT_BEST_VIDEO_FORMAT),
        "--add-metadata",
        "--abort-on-unavailable-fragment",
        "--compat-options",
        "abort-on-error",
    ];

    let bandwidth_limit = args
        .limit_bandwidth
        .as_ref()
        .or(platform_dl_options.bandwidth_limit.as_ref())
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

    if tmp_dir.is_none()
        && !platform_dl_options.skip_repair_date.unwrap_or(false)
        && !args.skip_repair_date
    {
        bail!("Cannot repair date in a non-temporary directory.\n\n{REPAIR_DATE_EXPLANATION}");
    }

    if !args.no_thumbnail && platform_dl_options.no_thumbnail != Some(true) {
        ytdl_args.push("--embed-thumbnail");

        if let Some(format) = &platform_dl_options.output_format {
            ytdl_args.push("--merge-output-format");
            ytdl_args.push(format);
        }
    }

    let cookies = args
        .cookies
        .as_ref()
        .or(platform_dl_options.cookies.as_ref());

    if let Some(cookies) = cookies {
        append_cookies_args(&mut ytdl_args, cookies)?;
    }

    if platform_dl_options.rate_limited == Some(true) || args.rate_limited {
        ytdl_args.push("--sleep-requests=3");
    }

    let mut filenaming = args
        .filenaming
        .as_deref()
        .unwrap_or(DEFAULT_FILENAMING)
        .to_owned();

    if args.index_prefix {
        let in_playlist = in_playlist.context(
            "Cannot add an index prefix as this video isn't part of a playlist download",
        )?;

        filenaming = format!(
            "{:0total_len$}. {filenaming}",
            in_playlist.index + 1,
            total_len = in_playlist.total.to_string().len()
        )
    };

    let dl_dir = match &tmp_dir {
        Some(tmp_dir) => tmp_dir.clone(),
        None => output_dir.clone(),
    };

    let ytdlp_output = dl_dir.join(format!("%(upload_date)s---{filenaming}"));

    ytdl_args.push("-o");
    ytdl_args.push(
        ytdlp_output
            .to_str()
            .context("Output directory contains invalid UTF-8 characters")?,
    );

    ytdl_args.push(url);

    info!(
        "> Downloading video {}",
        match &platform {
            Some(platform) => format!("from platform {}", platform.platform_name.bright_cyan()),
            None => "without a platform".bright_yellow().to_string(),
        }
    );

    if let Some(cookies) = cookies {
        match cookies {
            UseCookiesFrom::Browser(name) => {
                info!("| Using cookies from browser {}", name.bright_yellow())
            }
            UseCookiesFrom::File(path) => {
                info!("| Using cookies from file {}", path.bright_magenta())
            }
        }
    }

    if let Some(args) = &platform_dl_options.forward_ytdlp_args {
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

    let video_filename = video_file.file_name().unwrap().to_str().with_context(|| {
        format!(
            "Downloaded file name contains invalid UTF-8 characters: {}",
            video_file.display()
        )
    })?;

    let captured = EXTRACT_UPLOAD_DATE_REGEX
        .captures(video_filename)
        .with_context(|| {
            format!(
                "Failed to extract upload date from downloaded file name: {}",
                video_file.display()
            )
        })?;

    let video_upload_date = captured.name("date").unwrap().as_str();
    let video_filename = captured.name("filename").unwrap().as_str();

    let extracted_date =
        if !args.skip_repair_date && platform_dl_options.skip_repair_date != Some(true) {
            info!("| Extracting date from downloaded file");
            parse_date(&video_file, video_upload_date)?
        } else {
            None
        };

    info!(
        "> Moving the download file to output directory: {}",
        output_dir.to_string_lossy().bright_magenta()
    );

    let output_file = output_dir.join(video_filename);

    fs::copy(&video_file, &output_file).with_context(|| {
        format!(
            "Failed to move downloaded file: {}",
            video_file.to_string_lossy().bright_magenta()
        )
    })?;

    fs::remove_file(&video_file).with_context(|| format!("Failed to remove temporary download file at path: {}, directory will not be cleaned up",
        video_file.to_string_lossy().bright_magenta()
    ))?;

    if let Some(date) = extracted_date {
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

static EXTRACT_UPLOAD_DATE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(pomsky!(
        Start :date([Letter d]+) "---" :filename(.+) End
    ))
    .unwrap()
});

fn download_playlist_inner(
    playlist_url: &str,
    args: &SingleDlArgs,
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

        urls.push((
            url,
            SingleDlArgs {
                cookies: platform_config
                    .dl_options
                    .cookies
                    .clone()
                    .or(args.cookies.clone()),
                ..args.clone()
            },
        ));
    }

    download_inner(&urls, config, platform_matchers)
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

static AFTER_FAILURE_WAIT_DURATION_SECS: u64 = 5;
static RATE_LIMITED_WAIT_DURATION_SECS: u64 = 120;

static REPAIR_DATE_EXPLANATION: &str = r#"
By default, ytdl tries to write the videos' upload date to the downloaded files' metadata.

This requires specific support by the platform by the platform you're downloading from,
and also to use a temporary directory.

If you wish to disable this behaviour, use the `--skip-repair-date` option, or configure it
in your ytdl-config.json file.
"#;
