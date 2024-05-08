mod cmd;
mod constants;
mod repair_date;

pub use cmd::DlArgs;
pub use constants::*;

use crate::{
    config::{Config, PlatformDownloadOptions},
    dl::repair_date::{apply_mtime, repair_date},
    info, success,
    utils::{
        platforms::{
            find_platform, try_find_platform, FoundPlatform, PlatformsMatchers,
            ID_REGEX_MATCHING_GROUP_NAME,
        },
        shell::{run_cmd_bi_outs, ShellErrInspector},
        ytdlp::fetch_playlist,
    },
};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::{
    env, fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn download(
    args: DlArgs,
    config: &Config,
    platform_matchers: &PlatformsMatchers,
    inspect_dl_err: Option<ShellErrInspector>,
) -> Result<()> {
    download_inner(args, config, platform_matchers, inspect_dl_err)
}

fn download_inner(
    args: DlArgs,
    config: &Config,
    platform_matchers: &PlatformsMatchers,
    inspect_dl_err: Option<ShellErrInspector>,
) -> Result<()> {
    if args.no_platform && !args.skip_repair_date {
        bail!("Cannot repair date without a platform");
    }

    let mut videos = Vec::with_capacity(args.urls.len());

    for url in &args.urls {
        let platform = try_find_platform(url, config, platform_matchers)?;

        if let Some(platform) = &platform {
            if platform.is_playlist {
                if args.urls.len() > 1 {
                    bail!("Cannot mix playlist and non-playlist downloads");
                }

                return download_playlist_inner(
                    url,
                    &args,
                    config,
                    platform,
                    platform_matchers,
                    inspect_dl_err,
                );
            }
        }

        videos.push((url, platform));
    }

    let colored_total = videos.len().to_string().bright_yellow();

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

        download_single_inner(url, *platform, &args, config, inspect_dl_err, in_playlist)?;
    }

    Ok(())
}

// NOTE: ignores `args.url`
fn download_single_inner(
    url: &str,
    platform: Option<FoundPlatform>,
    args: &DlArgs,
    config: &Config,

    inspect_dl_err: Option<ShellErrInspector>,
    in_playlist: Option<PositionInPlaylist>,
) -> Result<()> {
    let video_id = platform
        .as_ref()
        .map(|platform| -> Result<String> {
            Ok(platform
                .matchers
                .id_from_video_url
                .captures(url)
                .context("Failed to extract video ID from URL using the platform's matcher")?
                .name(ID_REGEX_MATCHING_GROUP_NAME)
                .unwrap()
                .as_str()
                .to_string())
        })
        .transpose()?;

    let dl_options = platform
        .as_ref()
        .map(|p| &p.platform_config.dl_options)
        .unwrap_or(&PlatformDownloadOptions {
            bandwidth_limit: None,
            needs_checking: None,
            rate_limited: None,
            cookies_from_browser: None,
            skip_repair_date: Some(true),
            output_format: None,
            download_format: None,
            no_thumbnail: None,
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
        bail!("Cannot repair date in a non-temporary directory.");
    }

    if !args.no_thumbnail && dl_options.no_thumbnail != Some(true) {
        ytdl_args.push("--embed-thumbnail");

        if let Some(format) = &dl_options.output_format {
            ytdl_args.push("--merge-output-format");
            ytdl_args.push(format);
        }
    }

    let cookies_from_browser = args
        .cookies_from_browser
        .as_ref()
        .or(dl_options.cookies_from_browser.as_ref());

    if let Some(cookies_from_browser) = cookies_from_browser {
        ytdl_args.push("--cookies-from-browser");
        ytdl_args.push(cookies_from_browser);
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

    for arg in &args.forward {
        ytdl_args.push(arg);
    }

    info!(
        "> Downloading video {}{}",
        match &platform {
            Some(platform) => format!("from platform {}", platform.platform_name.bright_cyan()),
            None => "without a platform".bright_yellow().to_string(),
        },
        match cookies_from_browser {
            Some(name) => format!(" (with cookies from browser {})", name.bright_yellow()),
            None => String::new(),
        }
    );

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
    run_cmd_bi_outs(&config.yt_dlp_bin, &ytdl_args, inspect_dl_err)
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
            cookies_from_browser.map(String::as_str),
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
    inspect_dl_err: Option<ShellErrInspector>,
) -> Result<()> {
    let FoundPlatform {
        platform_name,
        platform_config,
        is_playlist,
        matchers: _,
    } = platform;

    assert!(is_playlist);

    info!("Fetching playlist's content...");

    let playlist = fetch_playlist(
        &config.yt_dlp_bin,
        playlist_url,
        args.cookies_from_browser.as_deref(),
    )
    .context("Failed to fetch the playlist's content")?;

    let colored_total = playlist.entries.len().to_string().bright_yellow();

    info!("Detected {} videos.", colored_total);
    info!("");

    let mut urls = Vec::with_capacity(playlist.entries.len());

    for video in &playlist.entries {
        if platform_config.redirect_playlist_videos == Some(true) {
            let platform = find_platform(&video.url, config, platform_matchers)?;

            let url = if platform.platform_name == *platform_name {
                video.url.clone()
            } else {
                let video_id = platform.matchers
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
            };

            urls.push(url);
        }
    }

    download_inner(
        DlArgs {
            urls,
            cookies_from_browser: platform_config
                .dl_options
                .cookies_from_browser
                .clone()
                .or(args.cookies_from_browser.clone()),
            ..args.clone()
        },
        config,
        platform_matchers,
        inspect_dl_err,
    )
}

struct PositionInPlaylist {
    index: usize,
    total: usize,
}
