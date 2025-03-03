use std::path::PathBuf;

use clap::Args;

use crate::config::UseCookiesFrom;

#[derive(Args, Clone, Default)]
pub struct DlArgs {
    #[clap(help = "URL(s) of the video/playlist/channel/... to download")]
    pub urls: Vec<String>,

    #[clap(
        long,
        long_help = "Don't require a registered platform in the config file\nRemoves the ability to download playlists and per-platform configuration"
    )]
    pub no_platform: bool,

    #[clap(short, long, help = "Custom YT-DLP format")]
    pub format: Option<String>,

    #[clap(
        long,
        help = "Download to a custom temporary directory instead of the default one"
    )]
    pub custom_temp_dir: Option<PathBuf>,

    #[clap(
        long,
        help = "Download in the current directory instead of using a temporary directory",
        conflicts_with = "custom_temp_dir"
    )]
    pub no_temp_dir: bool,

    #[clap(long, help = "Output directory")]
    pub output_dir: Option<PathBuf>,

    #[clap(long, help = "Custom YT-DLP filenaming")]
    pub filenaming: Option<String>,

    #[clap(
        long,
        help = "Prefix with the video's number in playlist (e.g. '01. <rest of the filename>')"
    )]
    pub index_prefix: bool,

    #[clap(long, help = "Limit the download bandwidth")]
    pub limit_bandwidth: Option<String>,

    #[clap(
        long,
        help = "Use cookies from a browser or a file",
        long_help = "'browser:<name>' to extract from a browser (e.g. 'firefox' or 'chrome'), 'file:/path/to/file' to use a file",
        value_parser = parse_cookies_arg
    )]
    pub cookies: Option<UseCookiesFrom>,

    #[clap(long, help = "Repair every videos' date after download")]
    pub skip_repair_date: bool,

    #[clap(long, help = "Don't download any thumbnail")]
    pub no_thumbnail: bool,

    #[clap(long, help = "Slow down requests for rate-limited platforms")]
    pub rate_limited: bool,

    #[clap(
        short,
        long,
        help = "Additional arguments to provide to YT-DLP",
        allow_hyphen_values = true
    )]
    pub forward_ytdlp_args: Vec<String>,
}

pub fn parse_cookies_arg(arg: &str) -> Result<UseCookiesFrom, String> {
    if let Some(browser_name) = arg.strip_prefix("browser:") {
        Ok(UseCookiesFrom::Browser(browser_name.to_owned()))
    } else if let Some(file_path) = arg.strip_prefix("file:") {
        Ok(UseCookiesFrom::File(file_path.to_owned()))
    } else {
        Err(format!("Invalid cookies source: {arg}"))
    }
}
