use crate::ytdlp::RawVideoInfos;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
pub struct Cache {
    pub entries: Vec<CacheEntry>,
    pub max_index: usize,
}

impl Cache {
    pub fn new(entries: Vec<CacheEntry>) -> Self {
        let max_index = match entries.iter().map(|entry| entry.index).max() {
            Some(index) => index + 1,
            None => 0,
        };

        Self { entries, max_index }
    }

    pub fn load_from_disk(path: &Path) -> Result<Self, String> {
        let cache =
            fs::read_to_string(path).map_err(|e| format!("Failed to read cache file: {e}"))?;

        serde_json::from_str(&cache).map_err(|e| format!("Failed to decode cache file: {e}"))
    }

    pub fn save_to_disk(&self, path: &Path) -> Result<(), String> {
        fs::write(
            path,
            serde_json::to_string_pretty(self)
                .map_err(|e| format!("Failed to serialize cache content: {e}"))?,
        )
        .map_err(|e| format!("Failed to write cache file: {e}"))
    }
}

#[derive(Serialize, Deserialize)]
pub struct CacheEntry {
    pub ie_key: String,
    pub id: String,
    pub title: String,
    pub url: String,
    pub index: usize,
    pub sync_dir: PathBuf,
}

impl CacheEntry {
    fn indexed(index: usize, video: PlatformVideo) -> Self {
        let PlatformVideo {
            raw,
            sync_dir,
            id,
            needs_checking: _,
        } = video;

        #[forbid(unused_variables)]
        let RawVideoInfos { ie_key, title, url } = raw;

        Self {
            ie_key,
            id,
            title,
            url,
            index,
            sync_dir,
        }
    }
}

impl From<(usize, PlatformVideo)> for CacheEntry {
    fn from((index, video): (usize, PlatformVideo)) -> Self {
        Self::indexed(index, video)
    }
}

pub struct PlatformVideo {
    pub raw: RawVideoInfos,
    pub sync_dir: PathBuf,
    pub id: String,
    pub needs_checking: bool,
}
