use clap::Args;

#[derive(Args)]
pub struct SyncArgs {
    #[clap(help = "URL to synchronize")]
    pub url: Option<String>,

    #[clap(long = "dry-run", help = "Simulate the synchronization")]
    pub dry_run: bool,
}
