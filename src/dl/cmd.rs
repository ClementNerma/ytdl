use std::path::PathBuf;

use clap::Args;

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

    #[clap(long, help = "Use cookies from the provided browser")]
    pub cookies_from_browser: Option<String>,

    #[clap(long, help = "Repair every videos' date after download")]
    pub skip_repair_date: bool,

    #[clap(long, help = "Don't download any thumbnail")]
    pub no_thumbnail: bool,

    #[clap(
        long,
        help = "Additional arguments to provide to YT-DLP",
        allow_hyphen_values = true
    )]
    pub forward: Vec<String>,
}
