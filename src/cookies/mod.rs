mod access;
mod cmd;

use std::{
    fs,
    io::{self, Read},
};

pub use access::existing_cookie_path;
use anyhow::{Context, Result};
pub use cmd::CookiesArgs;
use colored::Colorize;
use time::{format_description::well_known::Iso8601, OffsetDateTime};

use self::cmd::CookiesAction;
use crate::{config::Config, cookies::access::cookie_path, info, success, warn};

pub fn cookies(args: &CookiesArgs, config: &Config) -> Result<()> {
    match &args.action {
        CookiesAction::List => {
            let profiles = fs::read_dir(&config.cookies_dir)
                .context("Failed to read the cookies directory")?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, std::io::Error>>()?;

            info!(
                "Found {} profiles:",
                profiles.len().to_string().bright_yellow()
            );
            info!("");

            for profile in profiles {
                if !profile.is_file() {
                    warn!(
                        "Found a non-file item in the cookies directory: {}",
                        profile.to_string_lossy().bright_magenta()
                    );
                    continue;
                }

                info!(
                    "{} {}",
                    "*".bright_yellow(),
                    profile.file_name().unwrap().to_string_lossy()
                );
            }

            Ok(())
        }

        CookiesAction::Write(args) => {
            let path = cookie_path(&args.profile, config);

            if path.is_file() {
                warn!("Going to override cookie profile.");
            }

            info!("Reading from STDIN...");

            let mut raw_cookies = String::new();

            io::stdin()
                .read_to_string(&mut raw_cookies)
                .context("Failed to read raw cookies data from STDIN")?;

            info!("Converting cookies...");

            let mut output: Vec<String> = vec![];

            for (i, cookie) in raw_cookies.lines().enumerate() {
                if cookie.trim().is_empty() {
                    continue;
                }

                let converted = parse_raw_cookie(cookie).with_context(|| {
                    format!(
                        "Failed to parse line n°{}",
                        (i + 1).to_string().bright_yellow()
                    )
                })?;

                output.push(converted);
            }

            fs::write(&path, output.join("\n")).with_context(|| {
                format!(
                    "Failed to write cookie file at path: {}",
                    path.to_string_lossy().bright_magenta()
                )
            })?;

            success!("Succesfully wrote the new cookie file!");

            Ok(())
        }
    }
}

fn parse_raw_cookie(cookie: &str) -> Result<String> {
    let mut sections = cookie.trim().split('\t');

    let name = sections.next().unwrap();
    let value = sections.next().context("Missing value in cookie")?;
    let domain = sections.next().context("Missing domain name in cookie")?;
    let path = sections.next().context("Missing path in cookie")?;
    let expiration = sections.next().context("Missing expiration in cookie")?;
    let http_only = sections.next().context("Missing HTTP only in cookie")?;

    let domain = if domain.starts_with('.') {
        domain.to_string()
    } else {
        format!(".{}", domain)
    };

    let http_only = if http_only == "✓" { "TRUE" } else { "FALSE" };

    let expiration = if expiration == "Session" {
        OffsetDateTime::now_local().context("Failed to determine local date/time")?
    } else {
        OffsetDateTime::parse(expiration, &Iso8601::DEFAULT).with_context(|| {
            format!(
                "Failed to parse expiration date: {}",
                expiration.bright_yellow()
            )
        })?
    };

    let expiration = expiration.unix_timestamp();

    let output = format!("{domain}\tTRUE\t{path}\t{http_only}\t{expiration}\t{name}\t{value}");

    Ok(output)
}
