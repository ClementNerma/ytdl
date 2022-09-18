use crate::{
    config::PlatformConfig,
    error, info_inline,
    platforms::ID_REGEX_MATCHING_GROUP_NAME,
    shell::{run_cmd, run_custom_cmd},
    success,
    sync::VIDEO_ID_REGEX,
    warn,
};
use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use pomsky_macro::pomsky;
use regex::Regex;
use std::{path::Path, process::Command};

lazy_static! {
    static ref UPLOAD_DATE_REGEX: Regex = Regex::new(pomsky!(
        Start :year("20" [digit]{2}) :month([digit]{2}) :day([digit]{2}) End
    ))
    .unwrap();
}

pub fn repair_date<P: AsRef<Path>>(
    files: &[P],
    yt_dlp_bin: &Path,
    platform: &PlatformConfig,
    cookie_file: Option<&str>,
) -> Result<()> {
    let counter_len = files.len().to_string().len();
    let mut warnings = 0;
    let mut errors = 0;

    for (i, file) in files.iter().enumerate() {
        let file = file.as_ref();

        let file_name = match file.file_name().and_then(|file_name| file_name.to_str()) {
            Some(path) => path,
            None => {
                warn!(
                "| Skipping file: {} (failed to get filename or invalid UTF-8 characters found)",
                file.to_string_lossy().bright_magenta()
            );

                return Ok(());
            }
        };

        let video_id = match VIDEO_ID_REGEX.captures(file_name) {
            Some(m) => m.name(ID_REGEX_MATCHING_GROUP_NAME).unwrap().as_str(),
            None => {
                warn!(
                    "| Skipping file: {} (failed to extract video ID)",
                    file_name.bright_magenta()
                );

                return Ok(());
            }
        };

        info_inline!(
            "| Treating video {:>width$} / {}: {} [{}] ",
            (i + 1).to_string().bright_yellow(),
            files.len().to_string().bright_yellow(),
            video_id.bright_black(),
            file_name
                .chars()
                .take(50)
                .collect::<String>()
                .bright_magenta(),
            width = counter_len
        );

        let mut args = ["--get-filename", "-o", "%(upload_date)s"].to_vec();

        if let Some(cookie_file) = cookie_file {
            args.push("--cookies");
            args.push(cookie_file);
        }

        let url = format!("{}{}", platform.videos_url_prefix, video_id);
        args.push(&url);

        match run_cmd(yt_dlp_bin, &args) {
            Err(_) => {
                error!("FAILED");
                errors += 1;
                continue;
            }

            Ok(date) => {
                // Necessary as YT-DLP will output two newlines after the date
                let date = date.trim();

                if date == "NA" {
                    warn!("NO DATE FOUND");
                    warnings += 1;
                    continue;
                }

                if let Err(err) = set_ytdlp_upload_date(file, date) {
                    error!("FAILED TO SET DATE\n{}", err.to_string().bright_red());
                    errors += 1;
                    continue;
                }
            }
        }

        success!("OK");
    }

    if warnings > 0 {
        warn!("Emitted {warnings} warnings!");
    }

    if errors > 0 {
        error!("Emitted {errors} errors!");
        bail!("Failed with {errors} errors");
    }

    Ok(())
}

fn set_ytdlp_upload_date(file: &Path, date: &str) -> Result<()> {
    let m = UPLOAD_DATE_REGEX.captures(date).context("Invalid date")?;

    let year = m.name("year").unwrap().as_str().parse::<i32>().unwrap();
    let month = m.name("month").unwrap().as_str().parse::<u8>().unwrap();
    let day = m.name("day").unwrap().as_str().parse::<u8>().unwrap();

    // TODO: find a more proper way to do this
    run_custom_cmd(
        Command::new("touch")
            .arg(file)
            .arg("-m")
            .arg("-d")
            .arg(&format!("{:0>4}{:0>2}{:0>2}", year, month, day)),
    )
    .context("Failed to run 'touch' command for modification date")?;

    Ok(())
}
