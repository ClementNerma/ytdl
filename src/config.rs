use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Deserialize)]
pub struct Config {
    pub yt_dlp_bin: PathBuf,
    pub url_filename: String,
    pub cache_filename: String,
    pub auto_blacklist_filename: String,
    pub custom_blacklist_filename: String,
    pub platforms: HashMap<String, PlatformConfig>,
}

impl Config {
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
