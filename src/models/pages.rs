use scraper::{Html, Selector};
use url::Url;

pub struct ChapterParser;

impl ChapterParser {
    // Extract info from body of a paginated html page.
    pub fn get_images(html: &str) -> Result<Vec<Url>, ()> {
        // implement cascade image

        if html.contains("var dirPath = '") {
            //Implement more secure detection
            return Self::paginated(html);
        } else {
            return Self::cascade(html);
        }
    }

    fn paginated(html: &str) -> Result<Vec<Url>, ()> {
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
            .map(|url_tex| Url::parse(&url_tex).expect("parse error"))
            .collect();
        return Ok(urls);
    }

    fn cascade(html: &str) -> Result<Vec<Url>, ()> {
        let document = Html::parse_document(html);

        let image_selector = Selector::parse("img.viewer-img").unwrap();
        let images = document.select(&image_selector);
        let mut urls: Vec<Url> = Vec::new();

        for image in images {
            let url_text = image.attr("data-src").unwrap();
            let url_obj = Url::parse(&url_text).unwrap();

            urls.push(url_obj);
        }

        return Ok(urls);
    }
}
