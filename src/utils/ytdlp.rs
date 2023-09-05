use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::{config::Config, cookies::check_existing_cookie_path};

use super::shell::run_cmd;

#[derive(Deserialize)]
pub struct RawPlaylist {
    pub entries: Vec<RawVideoInfos>,
}

#[derive(Serialize, Deserialize)]
pub struct RawVideoInfos {
    pub ie_key: String,
    pub title: String,
    pub url: String,
}

pub fn check_version(bin: &Path) -> Result<String> {
    run_cmd(bin, &["--version"])
}

pub fn fetch_playlist(
    bin: &Path,
    url: &str,
    cookie_profile: Option<&str>,
    config: &Config,
) -> Result<RawPlaylist> {
    let mut args = vec![
        "-J".to_owned(),
        "--flat-playlist".to_owned(),
        url.to_owned(),
    ];

    if let Some(cookie_profile) = cookie_profile {
        let cookie_path = check_existing_cookie_path(cookie_profile, config)?;

        args.push("--cookies".to_owned());
        args.push(
            cookie_path
                .to_str()
                .context("Cookie path contains invalid UTF-8 characters")?
                .to_owned(),
        )
    }

    let output = run_cmd(bin, &args)?;

    serde_json::from_str::<RawPlaylist>(&output).with_context(|| {
        format!(
            "Failed to decode playlist, YT-DLP returned:\n\n{}",
            output.yellow()
        )
    })
}

pub fn check_availability(bin: &Path, url: &str) -> Result<bool> {
    // TODO: detect if error is caused by video being unavailable or by another error in YT-DLP
    Ok(run_cmd(bin, &["--get-url", url]).is_ok())
}
