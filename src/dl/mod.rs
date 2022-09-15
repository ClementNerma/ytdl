mod cmd;
mod constants;
mod repair_date;

pub use cmd::DlArgs;
pub use constants::*;

use crate::{
    config::Config,
    cookies::cookie_path,
    dl::repair_date::repair_date,
    info,
    platforms::{find_platform, PlatformsMatchers},
    shell::{run_cmd_bi_outs, ShellErrInspector},
    warn,
};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::{
    env, fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn download(
    args: &DlArgs,
    config: &Config,
    platform_matchers: Option<&PlatformsMatchers>,
    inspect_dl_err: Option<ShellErrInspector>,
) -> Result<()> {
    let mut ytdl_args = vec![
        "--format",
        args.format.as_deref().unwrap_or(DEFAULT_BEST_FORMAT),
        "--limit-rate",
        args.limit_bandwidth
            .as_deref()
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
            match fs::canonicalize(&output_dir)
                .context("Failed to canonicalize output directory")?
                .file_name()
            {
                Some(file_name) => file_name.to_string_lossy().bright_cyan(),
                None => "/".bright_red(),
            }
        )
    } else {
        output_dir.to_string_lossy().to_string()
    };

    let is_temp_dir_cwd = tmp_dir == cwd;

    if is_temp_dir_cwd && args.repair_date {
        bail!("Cannot repair date in a non-temporary directory.");
    }

    if !is_temp_dir_cwd {
        info!(
            "> Downloading first to temporary directory: {}",
            tmp_dir.to_string_lossy().bright_magenta()
        );
        info!(
            "> Then moving to provided final directory: {}",
            output_dir_display.bright_cyan()
        );
    }

    if !args.no_thumbnail {
        ytdl_args.push("--embed-thumbnail");

        if args.url.starts_with("https://www.youtube.com/")
            || args.url.starts_with("https://music.youtube.com/")
        {
            ytdl_args.push("--merge-output-format");
            ytdl_args.push("mkv");
        }
    }

    let cookie_file = args
        .cookie_profile
        .as_ref()
        .map(|profile| {
            let file = cookie_path(profile, config).with_context(|| {
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

    let output_with_filenaming =
        tmp_dir.join(args.filenaming.as_deref().unwrap_or(DEFAULT_FILENAMING));

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

    let repair_date_platform = platform_matchers
        .filter(|_| args.repair_date)
        .map(|matchers| find_platform(&args.url, config, matchers))
        .transpose()?;

    run_cmd_bi_outs(&config.yt_dlp_bin, &ytdl_args, inspect_dl_err)
        .context("Failed to run YT-DLP")?;

    if is_temp_dir_cwd {
        return Ok(());
    }

    let files = fs::read_dir(&tmp_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;

    if args.repair_date {
        info!("> Repairing date as requested");

        let (platform, matchers) = repair_date_platform.unwrap();

        repair_date(
            &files,
            &config.yt_dlp_bin,
            platform,
            matchers,
            cookie_file.as_deref(),
        )?;
    }

    info!(
        "> Moving [{}] file(s) to output directory: {}",
        files.len().to_string().bright_yellow(),
        output_dir.to_string_lossy().bright_magenta()
    );

    let mut can_cleanup = true;

    for (i, file) in files.iter().enumerate() {
        info!(
            "> Moving item {} / {}: {}",
            i + 1,
            files.len(),
            file.to_string_lossy().bright_black()
        );

        let dest_file_path = &fs::canonicalize(&output_dir)
            .unwrap()
            .join(file.file_name().unwrap());

        fs::copy(file, dest_file_path).with_context(|| {
            format!(
                "Failed to move downloaded file: {}",
                file.to_string_lossy().bright_magenta()
            )
        })?;

        if let Err(err) = fs::remove_file(file) {
            warn!(
                "Failed to remove temporary download file at path: {}, directory will not be cleaned up (cause: {})",
                file.to_string_lossy().bright_magenta(),
                err.to_string().bright_yellow()
            );

            can_cleanup = false;
        }
    }

    if can_cleanup {
        fs::remove_dir(&tmp_dir).with_context(|| {
            format!(
                "Failed to remove temporary directory at path: {}",
                tmp_dir.to_string_lossy().bright_magenta()
            )
        })?;
    }

    info!("Done!");

    Ok(())
}
