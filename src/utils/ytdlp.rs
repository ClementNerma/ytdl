use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

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
    cookies_from_browser: Option<&str>,
) -> Result<RawPlaylist> {
    let mut args = vec![
        "-J".to_owned(),
        "--flat-playlist".to_owned(),
        url.to_owned(),
    ];

    if let Some(cookies_from_browser) = cookies_from_browser {
        args.push("--cookies-from-browser".to_owned());
        args.push(cookies_from_browser.to_owned())
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
