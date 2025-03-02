use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use clap::Args;
use colored::Colorize;
use reqwest::{header, Url};
use serde::Deserialize;

use crate::{
    config::{Config, UseCookiesFrom},
    dl::{download, parse_cookies_arg, DlArgs},
    info, success,
    utils::{
        filenames::sanitize_filename,
        platforms::{build_platform_matchers, find_platform, FoundPlatform},
        ytdlp::{fetch_playlist, RawPlaylist},
    },
    warn,
};

#[derive(Args)]
pub struct AlbumArgs {
    #[clap(help = "URL of the playlist (or single track) to download")]
    pub url: String,

    #[clap(long, help = "Use cookies", value_parser = parse_cookies_arg)]
    pub cookies: Option<UseCookiesFrom>,
}

pub fn download_album(args: AlbumArgs, config: &Config, cwd: &Path) -> Result<()> {
    let AlbumArgs { url, cookies } = args;

    let platform_matchers = build_platform_matchers(config)?;

    let FoundPlatform {
        platform_config,
        is_playlist,
        platform_name: _,
        platform_matchers: _,
    } = find_platform(&url, config, &platform_matchers)?;

    if !is_playlist {
        bail!("Provided URL is a video, not a playlist!");
    }

    info!("|\n| Part 1/5: Fetching playlist...\n|\n");

    let RawPlaylist { entries } = fetch_playlist(
        &config.yt_dlp_bin,
        &url,
        platform_config
            .dl_options
            .cookies
            .as_ref()
            .or(cookies.as_ref()),
    )?;

    info!(
        "|\n| Part 2/5: Downloading {} tracks...\n|\n",
        entries.len().to_string().bright_yellow()
    );

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let tmp_dir = config
        .tmp_dir
        .join(format!("{}-{}-album", now.as_secs(), now.subsec_micros()));

    fs::create_dir(&tmp_dir).with_context(|| {
        format!(
            "Failed to create a temporary download directory at path: {}",
            tmp_dir.to_string_lossy().bright_magenta()
        )
    })?;

    let counter_len = entries.len().to_string().len();

    for (i, entry) in entries.iter().enumerate() {
        info!(
            "| Downloading track {:>width$} / {}: {}...",
            (i + 1).to_string().bright_yellow(),
            entries.len().to_string().bright_yellow(),
            entry.title.bright_magenta(),
            width = counter_len
        );

        info!(
            "| Track from {} at {}",
            entry.ie_key.bright_cyan(),
            entry.url.bright_green(),
        );

        download(
            DlArgs {
                urls: vec![entry.url.clone()],
                no_temp_dir: true,
                output_dir: Some(tmp_dir.clone()),
                format: Some("bestaudio".to_string()),
                forward: vec!["--write-info-json".to_string()],
                no_thumbnail: true,
                skip_repair_date: true,
                cookies: cookies.clone(),
                filenaming: Some(format!("{:0counter_len$}. %(title)s.%(ext)s", i + 1)),
                ..Default::default()
            },
            config,
            &platform_matchers,
        )?;

        info!("");
    }

    info!("|\n| Part 3/5: Analyzing tracks metadata...\n|\n");

    let mut dl_files = fs::read_dir(&tmp_dir)
        .context("Failed to read temporary download directory")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to iterate over content of the temporary download directory")?;

    dl_files.sort_by_key(|entry| entry.path());

    // Seal the list
    let dl_files = dl_files;

    let mut initial_track_metadata = None;
    let mut moves = vec![];

    for (i, dl_file) in dl_files
        .iter()
        .filter(|c| c.path().extension().unwrap() != "json")
        .enumerate()
    {
        let dl_file = dl_file.path();

        if dl_file.extension().unwrap() == "json" {
            continue;
        }

        info!(
            "| Analyzing track {:>counter_len$} / {}...",
            (i + 1).to_string().bright_yellow(),
            entries.len().to_string().bright_yellow(),
        );

        let mut json_filename = dl_file.file_stem().unwrap().to_os_string();
        json_filename.push(".info.json");

        let json_path = tmp_dir.join(&json_filename);

        let track_metadata =
            extract_json_track_metadata(Path::new(&json_path)).with_context(|| {
                format!(
                    "Failed to extract informations from JSON file: {}",
                    json_filename.to_string_lossy().bright_magenta()
                )
            })?;

        let TrackMetadata {
            album,
            uploader,
            track,
            thumbnails: _,
        } = &track_metadata;

        let album_dir = match initial_track_metadata {
            None => {
                let album_dir = cwd.join(format!(
                    "{} - {}",
                    sanitize_filename(uploader),
                    sanitize_filename(album)
                ));

                if !album_dir.exists() {
                    fs::create_dir(&album_dir).with_context(|| {
                        format!(
                            "Failed to create album directory at: {}",
                            album_dir.display()
                        )
                    })?;
                }

                initial_track_metadata = Some((track_metadata.clone(), album_dir.clone()));

                album_dir
            }

            Some((ref initial_mt, ref album_dir)) => {
                if album != &initial_mt.album {
                    bail!(
                        "Album mismatch: expected '{}', found '{}'",
                        initial_mt.album.bright_yellow(),
                        album.bright_yellow()
                    );
                }

                if uploader != &initial_mt.uploader {
                    warn!(
                        "Artist mismatch: expected '{}', found '{}'",
                        initial_mt.uploader.bright_yellow(),
                        uploader.bright_yellow()
                    );
                }

                album_dir.clone()
            }
        };

        let file_ext = dl_file.extension().unwrap().to_str().unwrap();

        let track_file = album_dir.join(format!(
            "{:0counter_len$}. {}.{file_ext}",
            i + 1,
            sanitize_filename(track)
        ));

        moves.push((dl_file, track_file));
    }

    let (initial_track_metadata, album_dir) =
        initial_track_metadata.context("No track found in the provided playlist!")?;

    info!("");
    info!("|\n| Part 4/5: Downloading album thumbnail...\n|\n");

    info!("| Reading playlist metadata...");

    let thumbnail = initial_track_metadata
        .thumbnails
        .into_iter()
        .filter_map(|thumb| {
            let TrackThumbnail { url, height, width } = thumb;

            width
                .zip(height)
                .map(|(width, height)| ValidThumbnail { url, height, width })
        })
        .filter(|thumb| {
            // HACK: Fix for Youtube returning non-existing URLs
            thumb.url.contains(".googleusercontent.com")
        })
        .max_by_key(|thumb| {
            // Converting to higher-capacity number to avoid overflows
            u64::from(thumb.width) * u64::from(thumb.height)
        });

    match thumbnail {
        None => warn!("Warning: album has no thumbnail!"),
        Some(ValidThumbnail {
            url,
            height: _,
            width: _,
        }) => {
            info!("| Downloading thumbnail at: {}", url.bright_magenta());

            let (thumbnail_path, thumbnail_ext) = download_thumbnail(&url, &tmp_dir)?;

            moves.push((
                thumbnail_path.clone(),
                album_dir.join(format!("cover{}", thumbnail_ext.unwrap_or_default())),
            ));
        }
    }

    info!("");
    info!("|\n| Part 5/5: Copying files to destination...\n|\n");

    for (dl_file, track_file) in moves {
        info!(
            "| Copying to: {}",
            track_file
                .strip_prefix(cwd)
                .unwrap()
                .to_string_lossy()
                .bright_magenta()
        );

        fs::copy(&dl_file, track_file).context("Failed to copy track file to destination")?;
    }

    fs::remove_dir_all(&tmp_dir).context("Failed to remove the temporary download directory")?;

    success!("Done!");

    Ok(())
}

fn extract_json_track_metadata(json_path: &Path) -> Result<TrackMetadata> {
    if !json_path.exists() {
        bail!("JSON information file was not found");
    }

    let json = fs::read_to_string(json_path).context("Failed to read the JSON file")?;

    let metadata = serde_json::from_str::<TrackMetadata>(&json)
        .context("Failed to deserialize the JSON file's content")?;

    Ok(metadata)
}

fn download_thumbnail(url: &str, to_dir: &Path) -> Result<(PathBuf, Option<String>)> {
    let url = Url::parse(url).context("Failed to parse thumbnail URL")?;

    let res = reqwest::blocking::get(url.clone()).context("Failed to fetch thumbnail")?;

    let res = res
        .error_for_status()
        .context("Failed to fetch thumbnail")?;

    let mime_type = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|header| std::str::from_utf8(header.as_bytes()).ok())
        .map(str::to_owned);

    let res = res.bytes().context("Failed to get thumbnail's body")?;

    let maybe_ext = mime_type
        .and_then(|mime_type| mime_guess::get_mime_extensions_str(&mime_type)?.first())
        .map(|ext| (*ext).to_owned())
        .or_else(|| {
            url.path()
                .split('.')
                .last()
                .map(str::to_lowercase)
                .filter(|ext| ext == "png" || ext == "jpg" || ext == "jpeg")
        })
        .map(|ext| format!(".{ext}"));

    println!("{maybe_ext:?}");

    let path = to_dir.join(format!(
        "thumbnail{}",
        maybe_ext.clone().unwrap_or_default()
    ));

    fs::write(&path, res).context("Failed to write thumbnail to disk")?;

    Ok((path, maybe_ext))
}

#[derive(Deserialize, Clone)]
struct TrackMetadata {
    album: String,
    // artist: String,
    uploader: String,
    track: String,
    // release_year: u16,
    thumbnails: Vec<TrackThumbnail>,
}

#[derive(Deserialize, Clone)]
struct TrackThumbnail {
    url: String,
    height: Option<u16>,
    width: Option<u16>,
}

#[derive(Clone)]
struct ValidThumbnail {
    url: String,
    height: u16,
    width: u16,
}
