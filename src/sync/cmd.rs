use clap::Args;

#[derive(Args)]
pub struct SyncArgs {
    #[clap(long = "dry-run", help = "Simulate the synchronization")]
    pub dry_run: bool,
}
