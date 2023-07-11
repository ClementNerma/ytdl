use clap::Args;

#[derive(Args)]
pub struct AlbumArgs {
    #[clap(help = "URL of the playlist (or single track) to download")]
    pub url: String,

    #[clap(long, help = "Use a registered cookie profile")]
    pub cookie_profile: Option<String>,
}
