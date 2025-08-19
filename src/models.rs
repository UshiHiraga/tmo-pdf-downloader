use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manga {
    // This info is extracted from the url.
    pub index: u32,
    // This info is extracted from the html file.
    pub title: String,
    pub chapters: Vec<Chapter>,
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

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Failed to parse URL: {0}")]
    InvalidUrl(String),

    #[error("This url doesn't point to zonatmo")]
    InvalidMangaUrl,

    #[error("Manga index was not in the URL. Expected format: '.../library/ID/...'")]
    MissingId,

    #[error("There was provided both origin values. Which one choose?")]
    ConfuseInstruction,

    #[error("There was not provided a origin value")]
    MissingOrigin,

    #[error("Missing cache for this manga")]
    MissingCache,
}
