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
        #[clap(help = "Entry to (un-)blacklist (syntax: Platform/ID)")]
        entry: String,
    },
}
