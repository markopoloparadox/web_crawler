use http_req::response::StatusCode;
use scraper::{selector, Html, Selector};

/*
    What to do: Implement web crawler that finds all links inside a specific domain
    How to do it: Make it simple and understandable
    How to improve this program:
        - Write it so it is multi-threaded/concurrent (async)
*/

fn main() {
    // Fixed domain
    const target_uri: &str = "https://www.foi.unizg.hr/en";

    let html = fetch_html_document(target_uri).unwrap();
    scrap_links(&html);
}

pub fn fetch_html_document(url: &str) -> Option<Html> {
    let mut response_body = Vec::new();
    let response = http_req::request::get(url, &mut response_body);

    if response.is_err() {
        println!("Unable to get the html document from url: {}", url);
    }

    let response = response.unwrap();
    if response.status_code() != StatusCode::new(200) {
        let code = response.status_code();
        let reason = response.reason();
        println!(
            "Unable to fetch url: {}. Status code: {} {}",
            url, code, reason
        );
        return None;
    }

    let document = String::from_utf8(response_body);
    if document.is_err() {
        println!("Unable to parse html document from url: {}", url);
        return None;
    }
    let document = document.unwrap();

    Some(Html::parse_document(document.as_str()))
}

pub fn scrap_links(html: &Html) -> Option<Vec<String>> {
    let selector = Selector::parse("a[href]").ok()?;
    let elements = html.select(&selector);
    for element in elements {
        let url = element.value().attr("href")?;
        println!("Url: {}", url);
    }

    None
}
