mod cmd;

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
pub use cmd::AlbumArgs;
use colored::Colorize;
use serde::Deserialize;

use crate::{
    config::Config,
    dl::{download, DlArgs},
    info,
    platforms::{build_platform_matchers, PlatformsMatchers},
    success, warn,
    ytdlp::{fetch_playlist, RawPlaylist},
};

pub fn download_album(args: AlbumArgs, config: &Config, cwd: &Path) -> Result<()> {
    let AlbumArgs {
        url,
        cookie_profile,
    } = args;

    let platform_matchers = build_platform_matchers(config)?;

    info!("|\n| Part 1/5: Fetching playlist...\n|\n");

    let RawPlaylist { entries } =
        fetch_playlist(&config.yt_dlp_bin, &url, cookie_profile.as_deref(), config)?;

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
                url: entry.url.clone(),
                no_temp_dir: true,
                output_dir: Some(tmp_dir.clone()),
                format: Some("bestaudio".to_string()),
                forward: vec!["--write-info-json".to_string()],
                no_thumbnail: true,
                skip_repair_date: true,
                cookie_profile: cookie_profile.clone(),
                ..Default::default()
            },
            config,
            &platform_matchers,
            None,
        )?;

        info!("");
    }

    info!("|\n| Part 3/5: Analyzing tracks metadata...\n|\n");

    let dl_files = fs::read_dir(&tmp_dir)
        .context("Failed to read temporary download directory")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to iterate over content of the temporary download directory")?;

    let mut initial_track_metadata = None;
    let mut moves = vec![];
    let mut treated_json_files = HashSet::new();

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

        treated_json_files.insert(json_path);

        let TrackMetadata {
            album,
            uploader,
            track,
        } = &track_metadata;

        let album_dir = match initial_track_metadata {
            None => {
                let album_dir = cwd.join(format!("{uploader} - {album}"));

                if !album_dir.exists() {
                    fs::create_dir(&album_dir).context("Failed to create album directory")?;
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
                    bail!(
                        "Artist mismatch: expected '{}', found '{}'",
                        initial_mt.uploader.bright_yellow(),
                        uploader.bright_yellow()
                    );
                }

                album_dir.clone()
            }
        };

        let file_ext = dl_file.extension().unwrap().to_str().unwrap();

        let track_file = album_dir.join(format!("{:0counter_len$}. {track}.{file_ext}", i + 1));

        moves.push((dl_file, track_file));
    }

    if initial_track_metadata.is_none() {
        bail!("No track found in playlist");
    }

    info!("");
    info!("|\n| Part 4/5: Downloading album thumbnail...\n|\n");

    info!("| Downloading playlist metadata...");

    let album_json_file = download_single_file(
        &tmp_dir,
        &url,
        cookie_profile.clone(),
        config,
        &platform_matchers,
        vec![
            "--flat-playlist".to_string(),
            "--write-info-json".to_string(),
            "--skip-download".to_string(),
        ],
    )?;

    let album_infos =
        fs::read_to_string(&album_json_file).context("Failed to read album informations file")?;

    let AlbumMetadata { thumbnails } = serde_json::from_str::<AlbumMetadata>(&album_infos)
        .context("Failed to parse album metadata")?;

    let thumbnail = thumbnails
        .iter()
        .filter(|thumb| {
            // HACK: Fix for Youtube returning non-existing URLs
            !(thumb.url.contains(".ytimg.com") && thumb.width > 1000 && thumb.height > 1000)
        })
        .max_by_key(|thumb| {
            // Converting to higher-capacity number to avoid overflows
            u64::from(thumb.width) * u64::from(thumb.height)
        });

    match thumbnail {
        None => warn!("Warning: album has no thumbnail!"),
        Some(AlbumThumbnail {
            url,
            height: _,
            width: _,
        }) => {
            info!("| Downloading thumbnail at: {}", url.bright_magenta());

            let thumbnail_file = download_single_file(
                &tmp_dir,
                url,
                cookie_profile,
                config,
                &platform_matchers,
                vec![],
            )?;

            moves.insert(
                0,
                (
                    thumbnail_file.clone(),
                    initial_track_metadata.unwrap().1.join(format!(
                        "cover.{}",
                        thumbnail_file.extension().unwrap().to_str().unwrap()
                    )),
                ),
            );
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

fn download_single_file(
    tmp_dir: &Path,
    url: &str,
    cookie_profile: Option<String>,
    config: &Config,
    platform_matchers: &PlatformsMatchers,
    forward: Vec<String>,
) -> Result<PathBuf> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let tmp_dir = tmp_dir.join(format!("{}-{}-single", now.as_secs(), now.subsec_micros()));

    fs::create_dir(&tmp_dir).context("Failed to create a temporary download directory")?;

    download(
        DlArgs {
            url: url.to_string(),
            no_temp_dir: true,
            output_dir: Some(tmp_dir.clone()),
            skip_repair_date: true,
            no_platform: true,
            cookie_profile,
            forward,
            ..Default::default()
        },
        config,
        platform_matchers,
        None,
    )?;

    let dl_files = fs::read_dir(&tmp_dir)
        .context("Failed to read temporary download directory")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to iterate over content of the temporary download directory")?;

    assert_eq!(dl_files.len(), 1, "Expected exactly one file");

    Ok(dl_files[0].path())
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

#[derive(Deserialize, Clone)]
struct TrackMetadata {
    album: String,
    // artist: String,
    uploader: String,
    track: String,
    // release_year: u16,
}

#[derive(Deserialize)]
struct AlbumMetadata {
    thumbnails: Vec<AlbumThumbnail>,
}

#[derive(Deserialize)]
struct AlbumThumbnail {
    url: String,
    height: u16,
    width: u16,
}
