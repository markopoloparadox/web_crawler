use http_req::response::StatusCode;
use scraper::Html;

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

    println!("Hello, world!");
}

pub fn fetch_html_document(uri: &str) -> Option<Html> {
    let mut response_body = Vec::new();
    let response = http_req::request::get(uri, &mut response_body);

    if response.is_err() {
        println!("Unable to get the html document from uri: {}", uri);
    }

    let response = response.unwrap();
    if response.status_code() != StatusCode::new(200) {
        let code = response.status_code();
        let reason = response.reason();
        println!(
            "Unable to fetch uri: {}. Status code: {} {}",
            uri, code, reason
        );
        return None;
    }

    let document = String::from_utf8(response_body);
    if document.is_err() {
        println!("Unable to parse html document from uri: {}", uri);
        return None;
    }
    let document = document.unwrap();

    Some(Html::parse_document(document.as_str()))
}
