use std::str::FromStr;

use crate::models::{Chapter, Manga, View};
use scraper::{Html, Selector};
use url::Url;

/// Extracts info from the url and body of a manga html page.
/// TODO: Implement err path.
pub fn manga_html_to_json(index: String, html: String) -> Result<Manga, ()> {
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

    return Ok(Manga {
        index,
        title,
        chapters,
    });
}

/// Extract info from body of a paginated html page.
pub fn html_paginated_json(html: String) -> Result<Vec<Url>, ()> {
    // First part of the url
    let search_string = "var dirPath = '";
    let start_index = html.find(search_string).unwrap();
    let after_start = start_index + search_string.len();
    let end_index = html[after_start..].find("'").unwrap();
    let full_end_index = after_start + end_index;
    let extract_url = &html[after_start..full_end_index];

    // Second extract ulist
    let start_delimiter = "JSON.parse('[";
    let end_delimiter = "]');";

    let start_ix = html.find(start_delimiter).unwrap();
    let after_start_2 = start_ix + start_delimiter.len();
    let end_ix = html[after_start_2..].find(end_delimiter).unwrap();
    let text_list = &html[after_start_2..after_start_2 + end_ix].replace("\"", "");
    let vec_names = text_list.split(",");

    let urls: Vec<Url> = vec_names
        .map(|nombre| format!("{}{}", extract_url, nombre))
        .map(|url_tex| Url::from_str(&url_tex).expect("parse error"))
        .collect();
    return Ok(urls);
}
