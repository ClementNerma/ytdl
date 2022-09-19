use crate::{
    config::PlatformConfig,
    error, error_anyhow, info_inline,
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
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

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
) -> Result<HashMap<PathBuf, UploadDate>> {
    let counter_len = files.len().to_string().len();
    let mut warnings = 0;
    let mut errors = 0;

    let mut dates = HashMap::new();

    for (i, file) in files.iter().enumerate() {
        let file = file.as_ref();

        assert!(
            file.is_file(),
            "Found a non-file item in repair date directory: {}",
            file.display()
        );

        let file_name = match file.file_name().and_then(|file_name| file_name.to_str()) {
            Some(path) => path,
            None => {
                warn!(
                "| Skipping file: {} (failed to get filename or invalid UTF-8 characters found)",
                file.to_string_lossy().bright_magenta()
            );

                continue;
            }
        };

        let video_id = match VIDEO_ID_REGEX.captures(file_name) {
            Some(m) => m.name(ID_REGEX_MATCHING_GROUP_NAME).unwrap().as_str(),
            None => {
                warn!(
                    "| Skipping file: {} (failed to extract video ID)",
                    file_name.bright_magenta()
                );

                continue;
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
            Err(err) => {
                error!("FAILED");
                error_anyhow!(err);
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

                match set_ytdlp_upload_date(file, date) {
                    Ok(date) => {
                        dates.insert(file.to_path_buf(), date);
                    }

                    Err(err) => {
                        error!("FAILED TO SET DATE\n{}", err.to_string().bright_red());
                        errors += 1;
                        continue;
                    }
                }
            }
        }

        success!("OK");
    }

    if warnings > 0 {
        warn!("Emitted {warnings} warnings!");
    }

    if errors > 0 {
        error!("> Emitted {errors} errors!");
        bail!("> Failed with {errors} errors");
    }

    if warnings == 0 {
        success!("> Successfully repaired dates!");
    }

    Ok(dates)
}

pub fn apply_mtime(file: &Path, date: UploadDate) -> Result<()> {
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
