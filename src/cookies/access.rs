use std::path::PathBuf;

use crate::config::Config;

pub fn existing_cookie_path(name: &str, config: &Config) -> Option<PathBuf> {
    let path = cookie_path(name, config);

    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

pub fn cookie_path(name: &str, config: &Config) -> PathBuf {
    config.profiles_dir.join(name)
}
