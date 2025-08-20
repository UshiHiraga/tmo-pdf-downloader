use reqwest::Error;
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue};
pub fn fetch(url: &String) -> Result<Response, Error> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let mut headers_map = HeaderMap::new();
    headers_map.insert("Referer", HeaderValue::from_static(""));
    return client
        .get(url)
        .headers(headers_map)
        .send()?
        .error_for_status();
}
