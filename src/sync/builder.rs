use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use walkdir::WalkDir;

use super::{
    blacklist::{blacklist_video, load_optional_blacklists, Blacklist},
    cache::{Cache, CacheEntry, PlatformVideo},
};
use crate::{
    config::{Config, ID_REGEX_MATCHING_GROUP_NAME},
    error, info, info_inline, success, warn,
    ytdlp::{check_availability, fetch_playlist},
};

lazy_static! {
    static ref VIDEO_ID: Regex =
        Regex::new(r"-(.*)\.(mp4|mkv|webm|mov|avi|mp3|ogg|flac|alac|aac|3gp|wav|aiff|dsf)$")
            .unwrap();
}

pub fn build_or_update_cache(sync_dir: &Path, config: &Config) -> Result<Cache, String> {
    let cache_path = sync_dir.join(&config.cache_filename);

    if !cache_path.exists() {
        let cache = build_cache(sync_dir, config)?;
        cache.save_to_disk(&cache_path)?;
        return Ok(cache);
    }

    let old_cache = Cache::load_from_disk(&cache_path)?;

    let old_cache_entries = old_cache.entries.len();

    let updated_cache = remove_downloaded_entries(old_cache)?;

    if updated_cache.entries.len() == old_cache_entries {
        info!("Successfully checked cache, nothing to update.");
    } else {
        info!(
            "Successfully checked and updated cache ({} => {} entries).",
            old_cache_entries.to_string().bright_yellow(),
            updated_cache.entries.len().to_string().bright_yellow()
        );

        updated_cache.save_to_disk(&cache_path)?;
    }

    Ok(updated_cache)
}

fn build_cache(sync_dir: &Path, config: &Config) -> Result<Cache, String> {
    info!("Looking for playlists...");

    let playlists = find_playlists(sync_dir, config)?;

    info!(
        "Found {} playlist(s) to treat.",
        playlists.len().to_string().bright_yellow()
    );

    let sync_dirs: HashSet<_> = playlists.iter().map(|p| p.sync_dir.clone()).collect();

    // Decode blacklists beforehand to ensure there won't be an error that will make the whole program fail
    // after all playlists have been fetched.
    let blacklists = sync_dirs
        .iter()
        .map(|dir| -> Result<(&PathBuf, Blacklist), String> {
            let merged_blacklists = load_optional_blacklists(&[
                &sync_dir.join(dir).join(&config.auto_blacklist_filename),
                &sync_dir.join(dir).join(&config.custom_blacklist_filename),
            ])?;

            Ok((dir, merged_blacklists))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;

    // Build directory indexes beforehand to ensure there won't be an error that will make the whole program fail
    // after all playlists have been fetched.
    let indexes = build_approximate_indexes(&sync_dirs)?;

    let videos = fetch_playlists(playlists, config)?;

    info!("Found a total of {} videos.", videos.len());

    let videos = videos.into_iter().filter(|video| {
        let blacklist = blacklists
            .get(&video.sync_dir)
            .expect("Internal consistency error: blacklist not found for given video");

        !blacklist.is_blacklisted(&video.raw.ie_key, &video.id)
    });

    let videos: Vec<_> = videos
        .filter(|video| !indexes.get(&video.sync_dir).expect("Internal consistency error: failed to get index for given video's sync. directory").contains(&video.id))
        .collect();

    info!(
        "Found {} videos to treat.",
        videos.len().to_string().bright_yellow()
    );

    let videos = check_videos_availability(sync_dir, videos, config)?;

    let entries = videos
        .into_iter()
        .enumerate()
        .map(CacheEntry::from)
        .collect::<Vec<_>>();

    Ok(Cache::new(entries))
}

fn find_playlists(sync_dir: &Path, config: &Config) -> Result<Vec<PlaylistUrl>, String> {
    let mut playlists = vec![];

    let sync_dir = fs::canonicalize(sync_dir)
        .map_err(|e| format!("Failed to canonicalize synchronization directory: {e}"))?;

    for item in WalkDir::new(&sync_dir) {
        let item = item
            .map_err(|e| format!("Failed to read directory entry while scanning playlists: {e}"))?;

        if let Some(name) = item.file_name().to_str() {
            if name == config.url_filename {
                let url = fs::read_to_string(item.path()).map_err(|e| {
                    format!(
                        "Failed to read playlist file at path {}: {e}",
                        item.path().to_string_lossy().bright_magenta()
                    )
                })?;

                let path = fs::canonicalize(item.path().parent().unwrap_or_else(|| Path::new("")))
                    .map_err(|e| {
                        format!("Failed to canonicalize synchronization directory: {e}")
                    })?;

                let relative_path = if path == sync_dir {
                    Path::new(".")
                } else {
                    path.strip_prefix(&sync_dir).map_err(|e| {
                        format!("Failed to determine video's sync. dir relatively to root sync. dir: {e}")
                    })?
                };

                playlists.push(PlaylistUrl {
                    sync_dir: relative_path.to_path_buf(),
                    url: url.trim().to_string(),
                });
            }
        }
    }

    if playlists.is_empty() {
        return Err("ERROR: No playlist found!".into());
    }

    Ok(playlists)
}

fn fetch_playlists(
    playlists: Vec<PlaylistUrl>,
    config: &Config,
) -> Result<Vec<PlatformVideo>, String> {
    let platform_matchers = build_platform_matchers(config)?;

    let mut parallel_fetching = true;

    for playlist in &playlists {
        let platform = config
            .platforms
            .iter()
            .find(|(ie_key, _)| {
                platform_matchers
                    .get(ie_key)
                    .expect("Internal consistency error: failed to get platform's matchers")
                    .platform_url_matcher
                    .is_match(&playlist.url)
            })
            .map(|(_, config)| config)
            .ok_or_else(|| {
                format!(
                    "Playlist has unregistered platform given its URL: {}",
                    playlist.url.bright_magenta()
                )
            })?;

        if platform.rate_limited {
            parallel_fetching = false;
        }
    }

    if !parallel_fetching {
        warn!(
            "Detected at least one platform with rate limiting, fetching playlists sequentially."
        );
    }

    let pb = ProgressBar::new(playlists.len() as u64).with_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>3}/{len:3} {eta_precise} {msg}")
            .expect("Invalid template provided for ProgressBar")
            .progress_chars("##-"),
    );

    pb.set_message("Starting to fetch...");
    pb.enable_steady_tick(Duration::from_secs(100));

    let remaining = AtomicUsize::new(playlists.len());
    let playlist_fetcher = |p: PlaylistUrl| {
        let playlist =
            fetch_playlist(&config.yt_dlp_bin, &p.url).map(|playlist| (p.sync_dir, playlist));

        let rem = remaining.fetch_sub(1, Ordering::SeqCst) - 1;

        pb.inc(1);
        pb.set_message(
            format!("{} playlist(s) remaining", rem.to_string().bright_yellow())
                .bright_blue()
                .to_string(),
        );

        playlist
    };

    let mut playlists_content = if parallel_fetching {
        playlists
            .into_par_iter()
            .map(playlist_fetcher)
            .collect::<Result<Vec<_>, _>>()?
    } else {
        playlists
            .into_iter()
            .map(playlist_fetcher)
            .collect::<Result<Vec<_>, _>>()?
    };

    pb.finish_with_message("Done!");

    playlists_content.sort_by(|a, b| {
        a.0.to_string_lossy()
            .to_lowercase()
            .cmp(&b.0.to_string_lossy().to_lowercase())
    });

    let total_videos = playlists_content
        .iter()
        .map(|(_, playlist)| playlist.entries.len())
        .sum();

    let mut entries = Vec::with_capacity(total_videos);

    for (path, playlist) in playlists_content {
        for video in playlist.entries {
            let platform = config.platforms.get(&video.ie_key).ok_or_else(|| {
                format!(
                    "Found unregistered platform (IE key) {} for video at URL {}",
                    video.ie_key.bright_yellow(),
                    video.url.bright_magenta()
                )
            })?;

            let matcher = platform_matchers.get(&video.ie_key).expect(
                "Internal consistency error: failed to get platform matchers for given video",
            );

            let matching = matcher
                .id_from_video_url
                .captures(&video.url)
                .ok_or_else(|| {
                    format!(
                        "Video URL does not match provided pattern for platform {}: {} in {}",
                        video.ie_key.bright_yellow(),
                        matcher.id_from_video_url.to_string().bright_cyan(),
                        video.url.bright_magenta(),
                    )
                })?;

            let id = matching.name(ID_REGEX_MATCHING_GROUP_NAME).ok_or_else(|| {
                format!(
                    "Inconsistency error: missing ID capture group {} in platform regex {}",
                    ID_REGEX_MATCHING_GROUP_NAME.bright_cyan(),
                    matcher.id_from_video_url.to_string().bright_yellow()
                )
            })?;

            entries.push(PlatformVideo {
                id: id.as_str().to_string(),
                raw: video,
                sync_dir: path.clone(),
                needs_checking: platform.needs_checking,
            });
        }
    }

    Ok(entries)
}

fn build_platform_matchers(
    config: &Config,
) -> Result<HashMap<&String, PlatformMatchingRegexes>, String> {
    config
        .platforms
        .iter()
        .map(|(ie_key, config)| {
            let regexes = PlatformMatchingRegexes {
                platform_url_matcher: Regex::new(&config.playlists_url_regex).map_err(|e| {
                    format!(
                        "Platform {} has an invalid regex for playlist URL matching: {}",
                        ie_key.bright_cyan(),
                        e.to_string().bright_yellow()
                    )
                })?,

                id_from_video_url: Regex::new(&config.videos_url_regex).map_err(|e| {
                    format!(
                        "Platform {} has an invalid regex for playlist URL matching: {}",
                        ie_key.bright_cyan(),
                        e.to_string().bright_yellow()
                    )
                })?,
            };

            let has_id_group = regexes.id_from_video_url.capture_names().any(|name| {
                name.filter(|name| name == &ID_REGEX_MATCHING_GROUP_NAME)
                    .is_some()
            });

            if !has_id_group {
                return Err(format!(
                    "Platform {}'s regex for playlist URL matching is missing the '{}' capture group: {}",
                    ie_key.bright_cyan(),
                    ID_REGEX_MATCHING_GROUP_NAME.bright_yellow(),
                    config.videos_url_regex.bright_yellow()
                ));
            }

            Ok((ie_key, regexes))
        })
        .collect::<Result<HashMap<_, _>, _>>()
}

fn build_approximate_indexes(
    dirs: &HashSet<PathBuf>,
) -> Result<HashMap<&PathBuf, HashSet<String>>, String> {
    info!("Building directory index...");

    let dirs_ids = dirs
        .into_par_iter()
        .map(|dir| build_approximate_index(dir).map(|ids| (dir, ids)))
        .collect::<Result<HashMap<_, _>, _>>()?;

    info!("{}", "Index is ready.".bright_black());

    Ok(dirs_ids)
}

fn build_approximate_index(dir: &Path) -> Result<HashSet<String>, String> {
    let mut ids = HashSet::new();

    for item in WalkDir::new(dir) {
        let item =
            item.map_err(|e| format!("Failed to read directory entry while building index: {e}"))?;
        let path = item.path();

        if !path.is_file() {
            continue;
        }

        let filename = match item.file_name().to_str() {
            Some(name) => name,
            None => {
                warn!(
                    "Ignoring file with non-UTF-8 name: {}",
                    item.file_name().to_string_lossy()
                );
                continue;
            }
        };

        if let Some(m) = VIDEO_ID.captures(filename) {
            let id = m.get(1).unwrap().as_str();

            if !id.contains('-') {
                // if !ids.insert(id.to_string()) {
                //     warn!("Two files were found with the same video identifier: {id}");
                // }

                ids.insert(id.to_string());
                continue;
            }

            let mut res = vec![];

            for segment in id.split('-').rev() {
                res.push(segment);
                ids.insert(res.iter().rev().cloned().collect::<Vec<_>>().join("-"));
            }
        }
    }

    Ok(ids)
}

fn check_videos_availability(
    sync_dir: &Path,
    videos: Vec<PlatformVideo>,
    config: &Config,
) -> Result<Vec<PlatformVideo>, String> {
    let to_check = videos.iter().filter(|video| video.needs_checking).count();

    if to_check == 0 {
        return Ok(videos);
    }

    info!(
        "Checking availability of {} videos...",
        to_check.to_string().bright_yellow()
    );

    let str_len = to_check.to_string().len();

    let mut available = vec![];

    let mut i = 0;

    let longest_dir_len = videos
        .iter()
        .map(|video| video.sync_dir.to_string_lossy().len())
        .max()
        .expect("Internal consistency error: failed to get the longest directory in videos list");

    for video in videos.into_iter() {
        if !video.needs_checking {
            continue;
        }

        i += 1;

        let counter_str = format!(
            "{} / {}",
            format!("{:>str_len$}", i).bright_yellow(),
            to_check.to_string().bright_yellow()
        );

        info_inline!(
            "| Checking video {counter_str} {:<longest_dir_len$} {}... ",
            video.sync_dir.to_string_lossy().bright_cyan(),
            format!("({})", video.id).bright_black()
        );

        if check_availability(&config.yt_dlp_bin, &video.raw.url)? {
            success!("OK");

            available.push(video);
        } else {
            error!("ERROR");

            blacklist_video(
                &sync_dir
                    .join(&video.sync_dir)
                    .join(&config.auto_blacklist_filename),
                &video.raw.ie_key,
                &video.id,
            )?;
        }
    }

    Ok(available)
}

fn remove_downloaded_entries(from: Cache) -> Result<Cache, String> {
    let sync_dirs = from
        .entries
        .iter()
        .map(|entry| entry.sync_dir.clone())
        .collect::<HashSet<_>>();

    let indexes = build_approximate_indexes(&sync_dirs)?;

    Ok(Cache::new(
        from.entries
            .into_iter()
            .filter(|video| !indexes.get(&video.sync_dir).expect("Internal consistency error: failed to get index for given video's sync. directory").contains(&video.id))
            .collect::<Vec<_>>(),
    ))
}

struct PlatformMatchingRegexes {
    platform_url_matcher: Regex,
    id_from_video_url: Regex,
}

struct PlaylistUrl {
    sync_dir: PathBuf,
    url: String,
}
