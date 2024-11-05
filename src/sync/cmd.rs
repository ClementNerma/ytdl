use clap::{Args, Subcommand};

#[derive(Args)]
pub struct SyncArgs {
    #[clap(subcommand)]
    pub action: SyncAction,
}

#[derive(Subcommand)]
pub enum SyncAction {
    Setup {
        #[clap(help = "URL to synchronize")]
        url: String,
    },

    Run {
        #[clap(long = "dry-run", help = "Simulate the synchronization")]
        dry_run: bool,
    },

    Blacklist {
        #[clap(help = "Platform the video belongs to")]
        platform: String,

        #[clap(help = "ID of the video to blacklist")]
        video_id: String,
    },
}
