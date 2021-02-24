use std::collections::HashSet;

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
    const DOMAIN_ADDRESS: &str = "https://www.foi.unizg.hr";

    let mut visited = HashSet::new();
    let mut not_visited: HashSet<String> = HashSet::new();

    visited.insert(DOMAIN_ADDRESS.to_owned());

    let html = fetch_html_document(DOMAIN_ADDRESS).unwrap();
    let links = scrap_links(DOMAIN_ADDRESS, &html).unwrap();

    for link in links.iter() {
        if !visited.contains(link) && !not_visited.contains(link) {
            not_visited.insert(link.to_owned());
        }
    }

    /*     println!("Visited: {:#?}", visited);
    println!("Not Visited: {:#?}", not_visited); */

    while let Some(url) = not_visited.iter().next() {
        println!("Url: {}", url);
        let url = url.clone();

        if let Some(html) = fetch_html_document(&url) {
            let links = scrap_links(DOMAIN_ADDRESS, &html).unwrap();

            for link in links.iter() {
                if !visited.contains(link) && !not_visited.contains(link) {
                    not_visited.insert(link.to_owned());
                }
            }
        }

        not_visited.remove(&url);
        visited.insert(url);
    }
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

    let elements = html.select(&selector);
    let links: Vec<String> = elements
        .filter_map(|x| x.value().attr("href"))
        .filter_map(|x| normalize_url(domain_address, x))
        .collect();

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
