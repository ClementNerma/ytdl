use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub url_filename: String,
    pub cache_filename: String,
    pub auto_blacklist_filename: String,
    pub custom_blacklist_filename: String,
    pub platforms: HashMap<String, PlatformConfig>,
}

impl Config {
    // pub fn load_from_disk(path: &Path) -> Result<Self, String> {
    //     let content =
    //         fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {e}"))?;

    //     Self::decode(&content)
    // }

    pub fn decode(input: &str) -> Result<Self, String> {
        serde_json::from_str(input).map_err(|e| format!("Failed to decode config file: {e}"))
    }
}

#[derive(Deserialize)]
pub struct PlatformConfig {
    pub playlists_url_regex: String,
    pub videos_url_regex: String,
    pub needs_checking: bool,
    pub rate_limited: bool,
}

pub static ID_REGEX_MATCHING_GROUP_NAME: &str = "ID";
