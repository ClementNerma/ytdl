use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Deserialize)]
pub struct Config {
    pub yt_dlp_bin: PathBuf,
    pub tmp_dir: PathBuf,
    pub url_filename: String,
    pub cache_filename: String,
    pub auto_blacklist_filename: String,
    pub custom_blacklist_filename: String,
    pub default_bandwidth_limit: String,
    pub platforms: HashMap<String, PlatformConfig>,
    pub cookie_profile_files: HashMap<String, PathBuf>,
}

impl Config {
    pub fn decode(input: &str) -> Result<Self> {
        serde_json::from_str(input).context("Failed to decode provided configuration")
    }
}

#[derive(Deserialize)]
pub struct PlatformConfig {
    pub playlists_url_regex: String,
    pub videos_url_regex: String,
    pub videos_url_prefix: String,
    pub bandwidth_limit: Option<String>,
    pub needs_checking: Option<bool>,
    pub rate_limited: Option<bool>,
    pub cookie_profile: Option<String>,
    pub repair_date: Option<bool>,
}
