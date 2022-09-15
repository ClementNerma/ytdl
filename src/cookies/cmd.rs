use clap::{Args, Subcommand};

#[derive(Args)]
pub struct CookiesArgs {
    #[clap(subcommand)]
    pub(super) action: CookiesAction,
}

#[derive(Subcommand)]
pub enum CookiesAction {
    #[clap(about = "List all cookie profiles")]
    List,

    #[clap(
        about = "Convert cookies to Netscape format (see long help for more infos)",
        long_about = "Convert cookies copy/pasted from Chrome's Application -> Storage -> Cookies -> [domain] table, into the Netscape cookies format used by tools like 'curl' or 'youtube-dl' / 'yt-dlp'."
    )]
    Write(CookiesRenewArgs),
}

#[derive(Args)]
pub struct CookiesRenewArgs {
    #[clap(help = "Name of the profile to create or update")]
    pub profile: String,
}
