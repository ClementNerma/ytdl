use super::cache::Cache;
use crate::info;
use colored::Colorize;

pub fn display_sync(cache: &Cache) {
    let max_index = cache
        .entries
        .iter()
        .map(|entry| entry.index)
        .max()
        .unwrap_or(0);

    let str_len = max_index.to_string().len();

    for entry in &cache.entries {
        let counter_str =
            format!("{:>str_len$} / {}", entry.index + 1, cache.max_index).bright_black();

        let sync_dir = entry.sync_dir.to_string_lossy();

        info!(
            "{} {} {}{}",
            counter_str,
            format!("[{}]", entry.id).bright_magenta(),
            if sync_dir == "." {
                String::new()
            } else {
                format!("{} ", sync_dir.bright_cyan())
            },
            entry.title.bright_yellow()
        );
    }
}
