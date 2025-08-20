use crate::errors::{CacheError, ContentTypeError};
use chrono::Utc;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manga {
    // This info is extracted from the url.
    pub content_type: ContentType,
    pub index: u32,
    pub normalized_title: String,
    // This info is extracted from the html file.
    pub title: String,
    pub chapters: Vec<Chapter>,
}

impl Manga {
    /// Extracts info from the url and body of a manga html page.
    /// TODO: Implement err path.
    pub fn from_html(url: Url, html: String) -> Result<Manga, ()> {
        let document = Html::parse_document(&html);

        // Extracts title from the body.
        let title_selector = Selector::parse("h1.element-title").unwrap();
        let title_node = document.select(&title_selector).next().unwrap();
        let title = title_node.text().next().unwrap().trim().to_string();

        // Extracts chapters from the body.
        let chapters_selector = Selector::parse("div#chapters li.upload-link").unwrap();
        let chapters_nodes = document.select(&chapters_selector);
        let mut chapters: Vec<Chapter> = Vec::new();

        for chapter_node in chapters_nodes {
            // Extracts chapter's title.
            let name_selector = Selector::parse("h4 a").unwrap();
            let name_node = chapter_node.select(&name_selector).next().unwrap();
            let name = name_node.text().last().unwrap().trim().to_string();

            // Extracts providers of the chapter.
            let providers_selector = Selector::parse("li.list-group-item").unwrap();
            let providers_nodes = chapter_node.select(&providers_selector);
            let mut views: Vec<View> = Vec::new();

            for provider_node in providers_nodes {
                let a_selector = Selector::parse("a").unwrap();
                let a_nodes = provider_node.select(&a_selector);

                let scan_name = a_nodes.clone().next().unwrap().text().next().unwrap();
                let url_view = a_nodes.clone().last().unwrap().attr("href").unwrap();
                views.push(View {
                    scan: scan_name.to_string(),
                    link: url_view.to_string(),
                });
            }

            chapters.push(Chapter { name, views });
        }

        // Get the path segments
        let segments: Vec<&str> = match url.path_segments() {
            Some(s) => s.collect(),
            None => {
                panic!("url should be valid");
            }
        };

        return Ok(Manga {
            content_type: ContentType::from_str(segments[1]).unwrap(),
            index: segments[2].parse::<u32>().unwrap(),
            normalized_title: segments[3].to_string(),
            title,
            chapters,
        });
    }

    pub fn from_cache(cache: &Path, index: &str) -> Result<Manga, CacheError> {
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
pub enum ContentType {
    Manga,
    Manhua,
    Manhwa,
    Novela,
    WebNovel,
    OneShot,
    Doujinshi,
    Oel,
}

impl FromStr for ContentType {
    type Err = ContentTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "manga" => Ok(Self::Manga),
            _ => Err(ContentTypeError),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chapter {
    pub name: String,
    pub views: Vec<View>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct View {
    pub scan: String,
    pub link: String,
}
