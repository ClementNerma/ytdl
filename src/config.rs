use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub yt_dlp_bin: PathBuf,
    pub cookies_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub url_filename: String,
    pub cache_filename: String,
    pub auto_blacklist_filename: String,
    pub custom_blacklist_filename: String,
    pub default_bandwidth_limit: String,
    pub platforms: HashMap<String, PlatformConfig>,
}

impl Config {
    pub fn decode(input: &str) -> Result<Self> {
        serde_json::from_str(input).context("Failed to decode provided configuration")
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformConfig {
    pub platform_url_matcher: String,
    pub videos_url_regex: String,
    pub videos_url_prefix: String,
    pub playlist_url_matchers: Option<Vec<String>>,
    pub bandwidth_limit: Option<String>,
    pub needs_checking: Option<bool>,
    pub rate_limited: Option<bool>,
    pub cookie_profile: Option<String>,
    pub skip_repair_date: Option<bool>,
    pub output_format: Option<String>,
}
