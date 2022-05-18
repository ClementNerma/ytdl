use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use colored::Colorize;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub struct Blacklist(pub Vec<BlacklistEntry>);

impl Blacklist {
    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn new(entries: Vec<BlacklistEntry>) -> Self {
        Self(entries)
    }

    pub fn decode(content: &str) -> Result<Self, String> {
        Ok(Self(
            content
                .trim()
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    BlacklistEntry::decode(line)
                        .map_err(|e| format!("Failed to decode line n°{}: {e}", i + 1))
                })
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }

    pub fn is_blacklisted(&self, ie_key: &str, video_id: &str) -> bool {
        self.0
            .iter()
            .any(|entry| entry.ie_key == ie_key && entry.video_id == video_id)
    }
}

pub struct BlacklistEntry {
    ie_key: String,
    video_id: String,
}

impl BlacklistEntry {
    pub fn decode(line: &str) -> Result<Self, &'static str> {
        let mut segments = line.split('/');

        let ie_key = segments.next().ok_or("IE key is missing")?.to_string();
        let id = segments.next().ok_or("Video ID is missing")?.to_string();

        if segments.next().is_some() {
            Err("Too many segments (/)")
        } else {
            Ok(Self {
                ie_key,
                video_id: id,
            })
        }
    }
}

pub fn load_blacklist_file(path: &Path) -> Result<Blacklist, String> {
    let str = fs::read_to_string(&path).map_err(|e| {
        format!(
            "Failed to read blacklist file at path '{}': {}",
            path.to_string_lossy().bright_magenta(),
            e
        )
    })?;

    Blacklist::decode(&str)
}

pub fn load_optional_blacklists(paths: &[&Path]) -> Result<Blacklist, String> {
    let blacklists = paths
        .par_iter()
        .map(|path| {
            if path.exists() {
                load_blacklist_file(path)
            } else {
                Ok(Blacklist::empty())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Blacklist::new(
        blacklists
            .into_iter()
            .flat_map(|blacklist| blacklist.0)
            .collect(),
    ))
}

pub fn blacklist_video(path: &Path, ie_key: &str, video_id: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Failed to create or open blacklist file: {e}"))?;

    let line = format!("{}/{}", ie_key, video_id);

    writeln!(file, "{line}").map_err(|e| format!("Failed to update blacklist file: {e}"))
}
