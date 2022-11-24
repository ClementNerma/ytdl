use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct PlatformConfig {
    /// Regex matching all valid URLs for this platform
    pub platform_url_matcher: String,

    /// Regex matching all valid video URLs for this platform,
    /// with a capture pattern for the video's ID
    pub videos_url_regex: String,

    /// Prefix URL which, when appended a video's ID, gives a valid video URL
    pub videos_url_prefix: String,

    /// List of regexes matching playlist URLs
    pub playlist_url_matchers: Option<Vec<String>>,

    /// Bandwidth limit (e.g. "20M" for 20 MB/s)
    pub bandwidth_limit: Option<String>,

    /// Indicate if videos need to be checked for availibility before being downloaded
    /// (Only used for synchronization)
    pub needs_checking: Option<bool>,

    /// Indicate if the platform is rate limited (= can't fetch multiple playlists at once)
    /// (Only used for synchronization)
    pub rate_limited: Option<bool>,

    /// Cookie profile to use
    pub cookie_profile: Option<String>,

    /// Disable repairing the video's date
    pub skip_repair_date: Option<bool>,

    /// Output format (e.g. "mkv")
    pub output_format: Option<String>,

    /// Download format (e.g. "bestaudio")
    pub download_format: Option<String>,

    /// Disable thumbnail downloading and embedding
    pub no_thumbnail: Option<bool>,

    /// Redirect videos inside playlists
    /// e.g. platform "A" containing videos of platform "B"
    /// would see B's videos redirected to A by simply changing the videos' URL
    /// to A's video prefix + B's video ID
    ///
    /// Concrete use case example: Youtube Music playlists contain Youtube video entries
    pub redirect_playlist_videos: Option<bool>,
}
