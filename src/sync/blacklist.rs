use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use anyhow::{bail, Context, Result};
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

    pub fn decode(content: &str) -> Result<Self> {
        Ok(Self(
            content
                .trim()
                .lines()
                .enumerate()
                .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
                .map(|(i, line)| {
                    BlacklistEntry::decode(line)
                        .with_context(|| format!("Failed to decode line n°{}", i + 1))
                })
                .collect::<Result<Vec<_>>>()?,
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
    pub fn decode(line: &str) -> Result<Self> {
        let mut segments = line.split('/');

        let ie_key = segments.next().context("IE key is missing")?.to_string();
        let id = segments.next().context("Video ID is missing")?.to_string();

        if segments.next().is_some() {
            bail!("Too many segments (/)")
        } else {
            Ok(Self {
                ie_key,
                video_id: id,
            })
        }
    }
}

pub fn load_blacklist_file(path: &Path) -> Result<Blacklist> {
    let str = fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read blacklist file at path '{}'",
            path.to_string_lossy().bright_magenta()
        )
    })?;

    Blacklist::decode(&str).with_context(|| {
        format!(
            "Failed to decode blacklist {}",
            path.to_string_lossy().bright_magenta()
        )
    })
}

pub fn load_optional_blacklists(paths: &[&Path]) -> Result<Blacklist> {
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

pub fn blacklist_video(path: &Path, ie_key: &str, video_id: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .context("Failed to create or open blacklist file")?;

    let line = format!("{}/{}", ie_key, video_id);

    writeln!(file, "{line}").context("Failed to update blacklist file")
}
