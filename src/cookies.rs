use std::path::PathBuf;

use crate::config::Config;

pub fn cookie_path(name: &str, config: &Config) -> Option<PathBuf> {
    let path = config.cookies_dir.join(name);

    if path.is_file() {
        Some(path)
    } else {
        None
    }
}
