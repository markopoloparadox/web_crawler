use http_req::response::StatusCode;
use scraper::{Html, Selector};
use url::Url;

/*
    What to do: Implement web crawler that finds all links inside a specific domain
    How to do it: Make it simple and understandable
    How to improve this program:
        - Write it so it is multi-threaded/concurrent (async)
*/

fn main() {
    // Fixed domain
    const DOMAIN_ADDRESS: &str = "https://www.foi.unizg.hr/en";

    let html = fetch_html_document(DOMAIN_ADDRESS).unwrap();
    let mut links = scrap_links(DOMAIN_ADDRESS, &html).unwrap();

    // Remove duplicates
    links.sort();
    links.dedup();
}

pub fn fetch_html_document(url: &str) -> Option<Html> {
    let mut response_body = Vec::new();
    let response = http_req::request::get(url, &mut response_body).ok()?;

    if response.status_code() != StatusCode::new(200) {
        let code = response.status_code();
        let reason = response.reason();
        println!(
            "Unable to fetch url: {}. Status code: {} {}",
            url, code, reason
        );
        return None;
    }

    let document = String::from_utf8(response_body).ok()?;
    Some(Html::parse_document(document.as_str()))
}

pub fn scrap_links(domain_address: &str, html: &Html) -> Option<Vec<String>> {
    let selector = Selector::parse("a[href]").ok()?;

    let mut links = Vec::new();
    let elements = html.select(&selector);
    for element in elements {
        let url = element.value().attr("href")?;
        if let Some(url) = normalize_url(domain_address, url) {
            links.push(url);
        }
    }

    Some(links)
}

pub fn normalize_url(domain_address: &str, url_source: &str) -> Option<String> {
    let url = Url::parse(url_source);

    if let Ok(url) = url {
        if url.has_host() && url.host_str().unwrap() == domain_address {
            return Some(url.to_string());
        }
    } else if url_source.starts_with('/') {
        return Some(domain_address.to_owned() + url_source);
    }

    None
}
