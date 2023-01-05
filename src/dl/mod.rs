mod cmd;
mod constants;
mod repair_date;

pub use cmd::DlArgs;
pub use constants::*;

use crate::{
    config::Config,
    cookies::existing_cookie_path,
    dl::repair_date::{apply_mtime, repair_date},
    info,
    platforms::{find_platform, FoundPlatform, PlatformsMatchers, ID_REGEX_MATCHING_GROUP_NAME},
    shell::{run_cmd_bi_outs, ShellErrInspector},
    success,
    ytdlp::fetch_playlist,
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
    download_inner(args, config, platform_matchers, inspect_dl_err, None)
}

fn download_inner(
    args: DlArgs,
    config: &Config,
    platform_matchers: &PlatformsMatchers,
    inspect_dl_err: Option<ShellErrInspector>,
    in_playlist: Option<VideoInPlaylist>,
) -> Result<()> {
    let FoundPlatform {
        platform_name,
        platform_config,
        matchers,
        is_playlist,
    } = find_platform(&args.url, config, platform_matchers)?;

    if is_playlist {
        info!("Fetching playlist's content...");

        let playlist = fetch_playlist(&config.yt_dlp_bin, &args.url)
            .context("Failed to fetch the playlist's content")?;

        let colored_total = playlist.entries.len().to_string().bright_yellow();

        info!("Detected {} videos.", colored_total);
        info!("");

        let mut entries = playlist.entries;

        for video in entries.iter_mut() {
            if platform_config.redirect_playlist_videos == Some(true) {
                let platform = find_platform(&video.url, config, platform_matchers)?;

                if platform.platform_name != platform_name {
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

                    video.url = format!("{}{}", platform_config.videos_url_prefix, video_id);
                }
            }
        }

        for (i, video) in entries.iter().enumerate() {
            info!(
                "> Downloading video {} / {colored_total}...",
                (i + 1).to_string().bright_yellow()
            );

            let cloned = args.clone();

            download_inner(
                DlArgs {
                    url: video.url.clone(),
                    cookie_profile: match &platform_config.cookie_profile {
                        Some(profile) => Some(profile.clone()),
                        None => cloned.cookie_profile.clone(),
                    },
                    ..cloned
                },
                config,
                platform_matchers,
                inspect_dl_err,
                Some(VideoInPlaylist {
                    index: i,
                    total: entries.len(),
                }),
            )?;

            info!("");
        }

        return Ok(());
    }

    let video_id = matchers
        .id_from_video_url
        .captures(&args.url)
        .context("Failed to extract video ID from URL using the platform's matcher")?
        .name(ID_REGEX_MATCHING_GROUP_NAME)
        .unwrap()
        .as_str()
        .to_string();

    let mut ytdl_args = vec![
        "--format",
        args.format
            .as_deref()
            .or(platform_config.download_format.as_deref())
            .unwrap_or(DEFAULT_BEST_FORMAT),
        "--limit-rate",
        args.limit_bandwidth
            .as_deref()
            .or(platform_config.bandwidth_limit.as_deref())
            .unwrap_or(&config.default_bandwidth_limit),
        "--add-metadata",
        "--abort-on-unavailable-fragment",
        "--compat-options",
        "abort-on-error",
    ];

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

    let tmp_dir = args.custom_tmp_dir.as_ref().unwrap_or(&config.tmp_dir);

    if !tmp_dir.is_dir() {
        fs::create_dir(tmp_dir).with_context(|| {
            format!(
                "Provided temporary directory does not exist at path: {}",
                tmp_dir.to_string_lossy().bright_magenta()
            )
        })?;
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let tmp_dir = tmp_dir.join(format!("{}-{}", now.as_secs(), now.subsec_micros()));

    let output_dir_display = if output_dir == Path::new(".") || output_dir == cwd {
        format!(
            ". ({})",
            cwd.file_name().unwrap().to_string_lossy().bright_cyan()
        )
    } else {
        output_dir.to_string_lossy().to_string()
    };

    let is_temp_dir_cwd = tmp_dir == cwd;

    if is_temp_dir_cwd && !args.skip_repair_date {
        bail!("Cannot repair date in a non-temporary directory.");
    }

    if !args.no_thumbnail && platform_config.no_thumbnail != Some(true) {
        ytdl_args.push("--embed-thumbnail");

        if let Some(format) = &platform_config.output_format {
            ytdl_args.push("--merge-output-format");
            ytdl_args.push(&format);
        }
    }

    let cookie_profile = args
        .cookie_profile
        .as_ref()
        .or(platform_config.cookie_profile.as_ref());

    let cookie_file = cookie_profile
        .map(|profile| {
            let file = existing_cookie_path(profile, config).with_context(|| {
                format!(
                    "The provided cookie profile '{}' was not found",
                    profile.bright_cyan()
                )
            })?;

            let file_path = file.to_str().context(
                "The provided profile's cookie file's path contains invalid UTF-8 characters",
            )?;

            if !file.is_file() {
                bail!(
                    "Provided profile's cookie file was not found at path: {}",
                    file_path.bright_magenta()
                );
            }

            Ok(file_path.to_string())
        })
        .transpose()?;

    if let Some(ref cookie_file) = cookie_file {
        ytdl_args.push("--cookies");
        ytdl_args.push(cookie_file);
    }

    let filenaming = args.filenaming.as_deref().unwrap_or(DEFAULT_FILENAMING);

    let output_with_filenaming = tmp_dir.join(if args.index_prefix {
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

    ytdl_args.push(&args.url);

    for arg in &args.forward {
        ytdl_args.push(arg);
    }

    info!(
        "> Downloading video from platform {}{}",
        platform_name.bright_cyan(),
        match cookie_profile {
            Some(name) => format!(" (with cookie profile {})", name.bright_yellow()),
            None => String::new(),
        }
    );

    if !is_temp_dir_cwd {
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

    if is_temp_dir_cwd {
        return Ok(());
    }

    let mut files =
        fs::read_dir(&tmp_dir).context("Failed to read the temporary download directory")?;

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

    let repair_dates = if !args.skip_repair_date && platform_config.skip_repair_date != Some(true) {
        info!("> Repairing date as requested");

        repair_date(
            &video_file,
            &video_id,
            &config.yt_dlp_bin,
            platform_config,
            cookie_file.as_deref(),
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

    fs::remove_dir(&tmp_dir).with_context(|| {
        format!(
            "Failed to remove temporary directory at path: {}",
            tmp_dir.to_string_lossy().bright_magenta()
        )
    })?;

    success!("> Done!");

    Ok(())
}

struct VideoInPlaylist {
    index: usize,
    total: usize,
}
