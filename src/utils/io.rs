use crate::models::Manga;
use chrono::Utc;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Could not read the cache directory: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cache for this manga does not exist or is not valid.")]
    CacheNotFound,

    #[error("The cache file is expired.")]
    CacheExpired,

    #[error("Failed to parse JSON content: {0}")]
    Parse(#[from] serde_json::Error),
}

pub fn get_manga_cache(folder_path: &Path, manga_id: &str) -> Result<Manga, CacheError> {
    let mut latest_file_path: Option<PathBuf> = None;
    let mut latest_timestamp: u64 = 0;
    let current_timestamp = Utc::now().timestamp() as u64;
    let fifteen_days_in_seconds: u64 = 15 * 24 * 3600;

    let cache_folder_entries = fs::read_dir(folder_path)?;
    for entry in cache_folder_entries {
        let path = entry?.path();

        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(res) => res,
            None => continue,
        };

        if !file_name.starts_with(&format!("{}-", manga_id)) {
            continue;
        }

        let parts: Vec<&str> = file_name.split('-').collect();

        if parts.len() <= 1 {
            continue;
        }

        let file_timestamp = match parts[1].parse::<u64>() {
            Ok(res) => res,
            Err(_) => continue,
        };

        if file_timestamp > latest_timestamp {
            latest_timestamp = file_timestamp;
            latest_file_path = Some(path);
        }
    }

    let latest_file_path = latest_file_path.ok_or(CacheError::CacheNotFound)?;

    if current_timestamp.abs_diff(latest_timestamp) > fifteen_days_in_seconds {
        return Err(CacheError::CacheExpired);
    };

    let contents = fs::read_to_string(latest_file_path)?;
    let json = serde_json::from_str(&contents)?;
    return Ok(json);
}

pub fn set_manga_cache(folder_path: &Path, id: &str, body: &Manga) -> Result<(), CacheError> {
    fs::create_dir_all(&folder_path)?;
    let current_timestamp = Utc::now().timestamp() as u64;
    let file_name = format!("{}-{}", id, current_timestamp);
    let json = serde_json::to_string_pretty(&body)?;
    fs::write(folder_path.join(&file_name), &json)?;
    return Ok(());
}
