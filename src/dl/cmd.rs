use std::path::PathBuf;

use clap::Args;

#[derive(Args)]
pub struct DlArgs {
    #[clap(help = "URL of the video/playlist/channel/... to download")]
    pub url: String,

    #[clap(short, long, help = "Custom YT-DLP format")]
    pub format: Option<String>,

    #[clap(
        long,
        help = "Download to a custom temporary directory instead of the default one"
    )]
    pub custom_tmp_dir: Option<PathBuf>,

    #[clap(long, help = "Output directory")]
    pub output_dir: Option<PathBuf>,

    #[clap(long, help = "Custom YT-DLP filenaming")]
    pub filenaming: Option<String>,

    #[clap(long, help = "Limit the download bandwidth")]
    pub limit_bandwidth: Option<String>,

    #[clap(long, help = "Use a registered cookie profile")]
    pub cookie_profile: Option<String>,

    #[clap(long, help = "Repair every videos' date after download")]
    pub repair_date: bool,

    #[clap(long, help = "Don't download any thumbnail")]
    pub no_thumbnail: bool,

    #[clap(long, help = "Additional arguments to provide to YT-DLP")]
    pub forward: Vec<String>,
}
