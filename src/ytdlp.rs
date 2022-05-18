use std::{
    io::{stderr, stdout, Write},
    process::Command,
};

use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::fail;

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

// pub struct IndexedVideoInfos {
//     pub raw: RawVideoInfos,
//     pub index: usize,
// }

static YT_DLP_CMD: &str = "yt-dlp";

fn flush_stdout() {
    stdout()
        .flush()
        .unwrap_or_else(|e| fail!("Failed to flush STDOUT: {e}"));

    stderr()
        .flush()
        .unwrap_or_else(|e| fail!("Failed to flush STDERR: {e}"));
}

fn call_yt_dlp(args: &[&str]) -> Result<String, String> {
    flush_stdout();

    let cmd = Command::new(YT_DLP_CMD)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run command: {e}"))?;

    flush_stdout();

    if !cmd.status.success() {
        let status_code = match cmd.status.code() {
            Some(code) => code.to_string(),
            None => String::from("<unknown code>"),
        };

        return Err(format!(
            "Failed to run command (status code = {}).\n\nArguments: {}\n\nProgram output:\n\n{}",
            status_code.bright_yellow(),
            args.iter()
                .map(|arg| format!("'{}'", arg.bright_yellow())
                    .bright_cyan()
                    .to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .bright_yellow(),
            String::from_utf8_lossy(&cmd.stderr).bright_yellow()
        ));
    }

    let output = std::str::from_utf8(&cmd.stdout)
        .map_err(|e| format!("Failed to decode command output as UTF-8: {e}"))?;

    Ok(output.to_string())
}

pub fn check_version() -> Result<String, String> {
    call_yt_dlp(&["--version"])
}

pub fn fetch_playlist(url: &str) -> Result<RawPlaylist, String> {
    let output = call_yt_dlp(&["-J", "--flat-playlist", url])?;

    serde_json::from_str::<RawPlaylist>(&output).map_err(|e| {
        format!(
            "Failed to decode playlist: {e}\n\nYT-DLP returned:\n\n{}",
            output.yellow()
        )
    })
}

pub fn check_availability(url: &str) -> Result<bool, String> {
    // TODO: detect if error is caused by video being unavailable or by another error in YT-DLP

    Ok(call_yt_dlp(&["--get-url", url]).is_ok())
}
