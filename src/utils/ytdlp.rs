use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::config::UseCookiesFrom;

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
    cookies: Option<&UseCookiesFrom>,
) -> Result<RawPlaylist> {
    let mut args = vec!["-J", "--flat-playlist", url];

    if let Some(cookies) = cookies {
        append_cookies_args(&mut args, cookies)?;
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

pub fn append_cookies_args<'a>(
    ytdlp_args: &mut Vec<&'a str>,
    cookies: &'a UseCookiesFrom,
) -> Result<()> {
    match cookies {
        UseCookiesFrom::Browser(browser) => {
            // TODO: ensure browser exists

            ytdlp_args.push("--cookies-from-browser");
            ytdlp_args.push(browser);
        }

        UseCookiesFrom::File(file) => {
            if !Path::new(file).is_file() {
                bail!("Cookie file does not exist: {}", file);
            }

            ytdlp_args.push("--cookies");
            ytdlp_args.push(file);
        }
    }

    Ok(())
}
