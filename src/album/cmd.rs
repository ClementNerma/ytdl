use clap::Args;

#[derive(Args)]
pub struct AlbumArgs {
    #[clap(help = "URL of the playlist (or single track) to download")]
    pub url: String,

    #[clap(long, help = "Use cookies from the provided browser")]
    pub cookies_from_browser: Option<String>,
}
