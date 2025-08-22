use crate::errors::{CacheError, SerieParseError};
use chrono::Utc;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SerieUrlInfo {
    pub url: String,
    pub index: u32,
    pub slug: String,
    pub is_oneshot: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Serie {
    // This info is extracted from the url.
    pub url_info: SerieUrlInfo,
    // This info is extracted from the html file.
    pub title: String,
    pub chapters: Vec<Chapter>,
}


impl Serie {
    /// Extracts info from the url and body of a manga html page.
    pub fn from_html(url_info: SerieUrlInfo, html: &str) -> Result<Serie, SerieParseError> {
        let document = Html::parse_document(&html);

        // Extracts title from the body.
        let title_selector = Selector::parse("h1.element-title").expect("Selector is hardcoded.");
        let Some(title_node) = document.select(&title_selector).next() else {
            return Err(SerieParseError::MissingTitle);
        };
        let title = match title_node.text().next() {
            Some(text) => text.trim().to_string(),
            None => return Err(SerieParseError::MissingTitle),
        };

        // If the serie is a one_shot we must extract providers in a different way.
        if url_info.is_oneshot {
            let providers_selector =
                Selector::parse("li.list-group-item").expect("Selector is hardcoded.");
            let provider_nodes = document.select(&providers_selector);

            let providers: Vec<Provider> = provider_nodes
                .filter_map(|j| Provider::from_oneshot_provider_fragment(j).ok())
                .collect();

            let chapters = vec![Chapter {
                number: (0, 0),
                name: title.clone(),
                providers,
            }];

            return Ok(Serie {
                url_info,
                title,
                chapters,
            });
        }

        // Extracts chapters from the body.
        let chapters_selector =
            Selector::parse("div#chapters li.upload-link").expect("Selector is hardcoded.");
        let chapters_nodes = document.select(&chapters_selector);
        let mut chapters: Vec<Chapter> = Vec::new();

        for chapter_node in chapters_nodes.rev() {
            // Extracts chapter's title.
            let name_selector = Selector::parse("h4 a").expect("Selector is hardcoded.");
            let Some(name_node) = chapter_node.select(&name_selector).next() else {
                return Err(SerieParseError::MissingTitle);
            };
            let name = match name_node.text().next() {
                Some(text) => text.trim().to_string(),
                None => return Err(SerieParseError::MissingTitle),
            };

            // Parses title to extract number
            let regex = Regex::new(r"Capítulo\s+(\d+)\.(\d+)").expect("Regex is hardcoded.");
            let Some(numbers) = regex.captures(&name) else {
                return Err(SerieParseError::MissingTitle);
            };
            let Ok(complete_part) = numbers
                .get(1)
                .expect("It selects the first capture group.")
                .as_str()
                .parse::<u32>()
            else {
                return Err(SerieParseError::MissingTitle);
            };
            let Ok(decimal_part) = numbers
                .get(2)
                .expect("It selects the second capture group.")
                .as_str()
                .parse::<u32>()
            else {
                return Err(SerieParseError::MissingTitle);
            };

            let number = (complete_part, decimal_part);

            // Extracts providers of the chapter.
            let providers_selector =
                Selector::parse("li.list-group-item").expect("Selector is hardcoded.");
            let providers_nodes = chapter_node.select(&providers_selector);

            let providers: Vec<Provider> = providers_nodes
                .filter_map(|j| Provider::from_serie_provider_fragment(j).ok())
                .collect();

            chapters.push(Chapter {
                number,
                name,
                providers,
            });
        }

        return Ok(Serie {
            url_info,
            title,
            chapters,
        });
    }

    pub fn from_cache(cache: &Path, index: &str) -> Result<Serie, CacheError> {
        let mut latest_file_path: Option<PathBuf> = None;
        let mut latest_timestamp: u64 = 0;
        let current_timestamp = Utc::now().timestamp() as u64;
        let fifteen_days_in_seconds: u64 = 15 * 24 * 3600;

        let cache_folder_entries = fs::read_dir(cache)?;

        for entry in cache_folder_entries {
            let path = entry?.path();

            if !path.is_file() {
                continue;
            }

            let file_name = match path.file_name().and_then(|name| name.to_str()) {
                Some(res) => res,
                None => continue,
            };

            if !file_name.starts_with(&format!("{}-", index)) {
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

    pub fn to_cache(&self, cache: &Path, index: &str) -> Result<(), CacheError> {
        fs::create_dir_all(&cache)?;
        let current_timestamp = Utc::now().timestamp() as u64;
        let file_name = format!("{}-{}", index, current_timestamp);
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(cache.join(&file_name), &json)?;
        return Ok(());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chapter {
    pub number: (u32, u32),
    pub name: String,
    pub providers: Vec<Provider>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Provider {
    pub scan: String,
    pub link: String,
}

impl Provider {
    fn from_oneshot_provider_fragment(frag: ElementRef) -> Result<Self, SerieParseError> {
        // Extract scan info.
        let scan_selector = Selector::parse("span").expect("Selector is hardcoded.");
        let Some(scan_node) = frag.select(&scan_selector).next() else {
            return Err(SerieParseError::MissingScan);
        };
        let scan = match scan_node.text().next() {
            Some(text) => text.trim().to_string(),
            None => return Err(SerieParseError::MissingScan),
        };

        // Extract view url.
        let a_selector = Selector::parse("a").expect("Selector is hardcoded.");
        let Some(a_node) = frag.select(&a_selector).next() else {
            return Err(SerieParseError::MissingScan);
        };
        let link = match a_node.attr("href") {
            Some(text) => text.to_string(),
            None => return Err(SerieParseError::MissingScan),
        };

        return Ok(Provider { scan, link });
    }

    fn from_serie_provider_fragment(frag: ElementRef) -> Result<Self, SerieParseError> {
        let a_selector = Selector::parse("a").expect("Selector is hardcoded.");

        // Extract scan info.
        let Some(scan_node) = frag.select(&a_selector).next() else {
            return Err(SerieParseError::MissingScan);
        };
        let scan = match scan_node.text().next() {
            Some(text) => text.trim().to_string(),
            None => return Err(SerieParseError::MissingScan),
        };

        // Extract view url.
        let Some(link_node) = frag.select(&a_selector).last() else {
            return Err(SerieParseError::MissingScan);
        };
        let link = match link_node.attr("href") {
            Some(text) => text.to_string(),
            None => return Err(SerieParseError::MissingScan),
        };

        return Ok(Provider { scan, link });
    }
}
