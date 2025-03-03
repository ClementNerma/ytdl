use crate::{utils::shell::run_custom_cmd, warn};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use pomsky_macro::pomsky;
use regex::Regex;
use std::{path::Path, process::Command, sync::LazyLock};

pub fn parse_date(file: &Path, date: &str) -> Result<Option<UploadDate>> {
    assert!(
        file.is_file(),
        "Found a non-file item in repair date directory: {}",
        file.display()
    );

    if date == "NA" {
        warn!("Could not get upload date for this video");
        return Ok(None);
    }

    let captured = UPLOAD_DATE_REGEX
        .captures(date)
        .with_context(|| format!("Invalid date: {}", date.bright_blue()))?;

    Ok(Some(UploadDate {
        year: captured
            .name("year")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap(),
        month: captured
            .name("month")
            .unwrap()
            .as_str()
            .parse::<u8>()
            .unwrap(),
        day: captured
            .name("day")
            .unwrap()
            .as_str()
            .parse::<u8>()
            .unwrap(),
    }))
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
            .arg(format!(
                "{:0>4}{:0>2}{:0>2}",
                date.year, date.month, date.day
            )),
    )
    .context("Failed to run 'touch' command for modification date")?;

    Ok(())
}

#[derive(Clone, Copy)]
pub struct UploadDate {
    year: i32,
    month: u8,
    day: u8,
}

static UPLOAD_DATE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(pomsky!(
        Start :year("20" [digit]{2}) :month([digit]{2}) :day([digit]{2}) End
    ))
    .unwrap()
});
