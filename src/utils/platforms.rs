use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use colored::Colorize;
use regex::Regex;

use crate::{
    config::{Config, PlatformConfig},
    regex::compile_pomsky,
    ytdlp::RawVideoInfos,
};

pub type PlatformsMatchers<'a> = HashMap<&'a String, PlatformMatchingRegexes>;

pub fn build_platform_matchers(config: &Config) -> Result<PlatformsMatchers> {
    config
        .platforms
        .iter()
        .map(|(ie_key, config)| {
            let platform = PlatformMatchingRegexes {
                platform_url_matcher: compile_pomsky(&config.platform_url_matcher).with_context(|| {
                    format!(
                        "Platform {} has an invalid regex for URL matching",
                        ie_key.bright_cyan(),
                    )
                })?,

                id_from_video_url: compile_pomsky(&config.videos_url_regex).with_context(|| {
                    format!(
                        "Platform {} has an invalid regex for videos URL matching",
                        ie_key.bright_cyan(),
                    )
                })?,
            };

            let has_id_group = platform.id_from_video_url.capture_names().any(|name| {
                name.filter(|name| name == &ID_REGEX_MATCHING_GROUP_NAME)
                    .is_some()
            });

            if !has_id_group {
                bail!(
                    "Platform {}'s regex for playlist URL matching is missing the '{}' capture group: {}",
                    ie_key.bright_cyan(),
                    ID_REGEX_MATCHING_GROUP_NAME.bright_yellow(),
                    config.videos_url_regex.bright_yellow()
                );
            }

            Ok((ie_key, platform))
        })
        .collect::<Result<HashMap<_, _>>>()
}

pub fn find_platform<'a, 'b>(
    url: &str,
    config: &'a Config,
    matchers: &'b PlatformsMatchers,
) -> Result<(&'a PlatformConfig, &'b PlatformMatchingRegexes)> {
    for (name, platform_config) in &config.platforms {
        let matcher = matchers
            .get(name)
            .context("Internal consistency error: failed to get platform's matcher")?;

        if matcher.platform_url_matcher.is_match(url) {
            return Ok((platform_config, matcher));
        }
    }

    bail!("No platform found for provided URL: {}", url.bright_cyan());
}

pub fn determine_video_id(
    video: &RawVideoInfos,
    platform_matchers: &PlatformsMatchers,
) -> Result<String> {
    let matcher = platform_matchers
        .get(&video.ie_key)
        .expect("Internal consistency error: failed to get platform matchers for given video");

    let matching = matcher
        .id_from_video_url
        .captures(&video.url)
        .with_context(|| {
            format!(
                "Video URL does not match provided pattern for platform {}: {} in {}",
                video.ie_key.bright_yellow(),
                matcher.id_from_video_url.to_string().bright_cyan(),
                video.url.bright_magenta(),
            )
        })?;

    let id = matching
        .name(ID_REGEX_MATCHING_GROUP_NAME)
        .with_context(|| {
            format!(
                "Inconsistency error: missing ID capture group {} in platform regex {}",
                ID_REGEX_MATCHING_GROUP_NAME.bright_cyan(),
                matcher.id_from_video_url.to_string().bright_yellow()
            )
        })?;

    Ok(id.as_str().to_string())
}

pub struct PlatformMatchingRegexes {
    pub platform_url_matcher: Regex,
    pub id_from_video_url: Regex,
}

pub static ID_REGEX_MATCHING_GROUP_NAME: &str = "id";
