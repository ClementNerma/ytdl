use crate::{
    config::PlatformConfig,
    error, info_inline,
    shell::{run_cmd, run_custom_cmd},
    success, warn,
};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use once_cell::sync::Lazy;
use pomsky_macro::pomsky;
use regex::Regex;
use std::{path::Path, process::Command};

static UPLOAD_DATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(pomsky!(
        Start :year("20" [digit]{2}) :month([digit]{2}) :day([digit]{2}) End
    ))
    .unwrap()
});

pub fn repair_date(
    file: &Path,
    video_id: &str,
    yt_dlp_bin: &Path,
    platform: &PlatformConfig,
    cookie_file: Option<&str>,
) -> Result<Option<UploadDate>> {
    assert!(
        file.is_file(),
        "Found a non-file item in repair date directory: {}",
        file.display()
    );

    let file_name = match file.file_name().and_then(|file_name| file_name.to_str()) {
        Some(path) => path,
        None => {
            bail!(
                "| Skipping file: {} (failed to get filename or invalid UTF-8 characters found)",
                file.to_string_lossy().bright_magenta()
            );
        }
    };

    info_inline!(
        "| Treating video {} [{}] ",
        video_id.bright_black(),
        file_name
            .chars()
            .take(50)
            .collect::<String>()
            .bright_magenta(),
    );

    let mut args = ["--get-filename", "-o", "%(upload_date)s"].to_vec();

    if let Some(cookie_file) = cookie_file {
        args.push("--cookies");
        args.push(cookie_file);
    }

    let url = format!("{}{}", platform.videos_url_prefix, video_id);
    args.push(&url);

    match run_cmd(yt_dlp_bin, &args) {
        Err(err) => {
            error!("FAILED");
            bail!("Failed to repair date: {err:?}");
        }

        Ok(date) => {
            // Necessary as YT-DLP will output two newlines after the date
            let date = date.trim();

            if date == "NA" {
                warn!("NO DATE FOUND");
                return Ok(None);
            }

            match set_ytdlp_upload_date(file, date) {
                Ok(date) => {
                    success!("OK");
                    Ok(Some(date))
                }

                Err(err) => {
                    error!("FAILED TO SET DATE");
                    bail!("Failed to repair date: {err:?}");
                }
            }
        }
    }
}

pub fn apply_mtime(file: &Path, date: UploadDate) -> Result<()> {
    // Guard to ensure the file exists, otherwise `touch` will create it!
    if !file.is_file() {
        bail!("Provided file does not exist!");
    }

    // TODO: find a more proper way to do this
    run_custom_cmd(
        Command::new("touch")
            .arg(file)
            .arg("-m")
            .arg("-d")
            .arg(&format!(
                "{:0>4}{:0>2}{:0>2}",
                date.year, date.month, date.day
            )),
    )
    .context("Failed to run 'touch' command for modification date")?;

    Ok(())
}

fn set_ytdlp_upload_date(file: &Path, date: &str) -> Result<UploadDate> {
    let m = UPLOAD_DATE_REGEX.captures(date).context("Invalid date")?;

    let date = UploadDate {
        year: m.name("year").unwrap().as_str().parse::<i32>().unwrap(),
        month: m.name("month").unwrap().as_str().parse::<u8>().unwrap(),
        day: m.name("day").unwrap().as_str().parse::<u8>().unwrap(),
    };

    apply_mtime(file, date)?;

    Ok(date)
}

#[derive(Clone, Copy)]
pub struct UploadDate {
    year: i32,
    month: u8,
    day: u8,
}
