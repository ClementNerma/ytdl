mod cmd;
mod constants;
mod repair_date;

use self::constants::{DEFAULT_BEST_FORMAT, DEFAULT_FILENAMING};
use crate::{
    config::Config,
    dl::repair_date::repair_date,
    info,
    platforms::{find_platform, PlatformMatchingRegexes},
    shell::{run_cmd_bi_outs, ShellErrInspector},
    warn,
};
use anyhow::{bail, Context, Result};
pub use cmd::DlArgs;
use colored::Colorize;
use std::{
    collections::HashMap,
    env, fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn download(
    args: &DlArgs,
    config: &Config,
    platform_matchers: Option<&HashMap<&String, PlatformMatchingRegexes>>,
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

    let tmp_dir = args
        .tmp_dir
        .as_ref()
        .map(|dir| {
            if !dir.is_dir() {
                bail!(
                    "Provided directory does not exist at path: {}",
                    dir.to_string_lossy().bright_magenta()
                );
            }

            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            Ok(dir.join(format!("{}-{}", now.as_secs(), now.subsec_micros())))
        })
        .transpose()?;

    let output_dir_display = if output_dir == Path::new(".") || output_dir == cwd {
        output_dir.to_string_lossy().to_string()
    } else {
        format!(
            ". ({})",
            match output_dir.file_name() {
                Some(file_name) => file_name.to_string_lossy().bright_magenta(),
                None => "/".bright_red(),
            }
        )
    };

    let is_temp_dir_cwd = tmp_dir.as_ref() == Some(&cwd);

    if is_temp_dir_cwd && args.repair_date {
        bail!("Cannot repair date in a non-temporary directory.");
    }

    if let Some(ref tmp_dir) = tmp_dir {
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
            let file = config.cookie_profile_files.get(profile).with_context(|| {
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

            Ok(file_path)
        })
        .transpose()?;

    if let Some(cookie_file) = cookie_file {
        ytdl_args.push("--cookies");
        ytdl_args.push(cookie_file);
    }

    let output_with_filenaming = tmp_dir
        .as_ref()
        .unwrap_or(&cwd)
        .join(args.filenaming.as_deref().unwrap_or(DEFAULT_FILENAMING));

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

    let tmp_dir = match (tmp_dir, is_temp_dir_cwd) {
        (Some(tmp_dir), false) => tmp_dir,
        _ => return Ok(()),
    };

    let files = fs::read_dir(&tmp_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;

    if args.repair_date {
        info!("> Repairing date as requested");

        let (platform, matchers) = repair_date_platform.unwrap();

        repair_date(&files, &config.yt_dlp_bin, platform, matchers, cookie_file)?;
    }

    info!(
        "> Moving [{}] files to output directory: {}",
        files.len().to_string().bright_yellow(),
        output_dir.to_string_lossy().bright_magenta()
    );

    let mut can_cleanup = true;

    for (i, file) in files.iter().enumerate() {
        let colored_file = file.to_string_lossy().bright_black();

        info!(
            "> Moving item {} / {}: {}",
            i + 1,
            files.len(),
            colored_file
        );

        fs::copy(file, &output_dir)
            .with_context(|| format!("Failed to move downloaded file: {}", colored_file))?;

        if let Err(err) = fs::remove_file(file) {
            warn!(
                "Failed to remove temporary download file at path: {}, directory will not be cleaned up (cause: {})",
                colored_file,
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
