use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::dl::VideoQuality;

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Shell command or filesystem path to the "yt-dlp" binary
    pub yt_dlp_bin: PathBuf,

    /// Path to the temporary download directory
    /// Relative to the configuration file's path or absolute
    pub tmp_dir: PathBuf,

    /// Name of the file containing the playlists URL for sync.
    pub url_filename: String,

    /// Name of the file containing the cache for sync.
    pub cache_filename: String,

    /// Name of the file containing the automatic blacklist for sync.
    pub auto_blacklist_filename: String,

    /// Name of the file containing the custom blacklist for sync.
    pub custom_blacklist_filename: String,

    /// Default bandwidth limit if none is provided by the platform and/or command-line arguments
    pub default_bandwidth_limit: Option<String>,

    /// List of all platforms to download from
    pub platforms: HashMap<String, PlatformConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            yt_dlp_bin: PathBuf::from("yt-dlp"),
            tmp_dir: std::env::temp_dir().join("ytdl"),
            url_filename: ".ytdlsync-url".to_string(),
            cache_filename: ".ytdlsync-cache".to_string(),
            auto_blacklist_filename: ".ytdlsync-blacklist".to_string(),
            custom_blacklist_filename: ".ytdlsync-custom-blacklist".to_string(),
            default_bandwidth_limit: None,
            platforms: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
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

    /// Redirect videos inside playlists
    /// e.g. platform "A" containing videos of platform "B"
    /// would see B's videos redirected to A by simply changing the videos' URL
    /// to A's video prefix + B's video ID
    ///
    /// Concrete use case example: Youtube Music playlists contain Youtube video entries
    pub redirect_playlist_videos: Option<bool>,

    /// Download options
    pub dl_options: PlatformDownloadOptions,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct PlatformDownloadOptions {
    /// Bandwidth limit (e.g. "20M" for 20 MB/s)
    pub bandwidth_limit: Option<String>,

    /// Indicate if videos need to be checked for availibility before being downloaded
    /// (Only used for synchronization)
    pub needs_checking: Option<bool>,

    /// Indicate if the platform is rate limited (= can't fetch multiple playlists at once)
    /// (Only used for synchronization)
    pub rate_limited: Option<bool>,

    /// Use cookies from the provided browser
    pub cookies: Option<UseCookiesFrom>,

    /// Disable repairing the video's date
    pub skip_repair_date: Option<bool>,

    /// Output format (e.g. "mkv")
    pub output_format: Option<String>,

    /// Default quality (e.g. "best-1080p")
    pub default_quality: Option<VideoQuality>,

    /// Raw YT-DLP download format for albums (e.g. "bestaudio")
    pub raw_album_format: Option<String>,

    /// Disable thumbnail downloading and embedding
    pub no_thumbnail: Option<bool>,

    /// Additional arguments to forward to YT-DLP
    pub forward_ytdlp_args: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UseCookiesFrom {
    #[serde(rename = "browser")]
    Browser(String),

    #[serde(rename = "file")]
    File(String),
}
