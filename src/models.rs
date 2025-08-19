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
