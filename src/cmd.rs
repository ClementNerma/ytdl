use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    #[clap(long, help = "Directory to synchronize")]
    pub sync_dir: PathBuf,

    #[clap(long, help = "Configuration as a JSON string")]
    pub config: String,

    #[clap(long, help = "Display the cache's content as a colored list")]
    pub display_colored_list: bool,
}
