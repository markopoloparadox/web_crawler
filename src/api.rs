use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tide::Body;
use tide::{Request, Response};
use url::Url;

use crate::State;

#[derive(Deserialize, Serialize)]
struct Domain {
    address: String,
}

pub async fn post_crawl(mut req: Request<State>) -> tide::Result {
    let domain: Domain = req.body_json().await?;
    println!("Domain name: {}", domain.address);

    test_run(&domain.address).await;

    Ok("Test".into())
}

pub async fn get_crawled_list(mut req: Request<State>) -> tide::Result {
    let crawls = req.state().links.read().unwrap();
    let id = req.param("id")?;

    if let Some(list) = crawls.get(id) {
        let mut res = Response::new(200);
        res.set_body(Body::from_json(list)?);
        return Ok(res);
    }

    Ok("Failed".into())
}

pub async fn get_crawled_count(mut req: Request<State>) -> tide::Result {
    let crawls = req.state().links.read().unwrap();
    let id = req.param("id")?;

    if let Some(list) = crawls.get(id) {
        let mut res = Response::new(200);
        res.set_body(Body::from_json(&list.len())?);
        return Ok(res);
    }

    Ok("Failed".into())
}

async fn test_run(domain_address: &str) {
    let mut visited = HashSet::new();
    let mut not_visited: HashSet<String> = HashSet::new();

    visited.insert(domain_address.to_owned());

    let links;
    {
        let html = fetch_html_document(domain_address).await.unwrap();
        links = scrap_links(domain_address, &html).unwrap();
    }

    for link in links.iter() {
        if !visited.contains(link) && !not_visited.contains(link) {
            not_visited.insert(link.to_owned());
        }
    }

    let visited = Arc::new(Mutex::new(visited));
    let not_visited = Arc::new(Mutex::new(not_visited));
    let domain_address = Arc::new(Mutex::new(domain_address.to_owned()));

    let mut handles = Vec::new();
    for _i in 0..100 {
        handles.push(async_std::task::spawn(test(
            domain_address.clone(),
            visited.clone(),
            not_visited.clone(),
        )));
    }

    for h in handles {
        h.await;
    }

    println!("Visited: {:#?}", visited);
}

pub async fn test(
    domain_address: Arc<Mutex<String>>,
    visited: Arc<Mutex<HashSet<String>>>,
    not_visited: Arc<Mutex<HashSet<String>>>,
) {
    const MAX_SLEEP_COUNT: u32 = 5;

    let mut sleep_count = 0;
    loop {
        let mut abc = None;
        {
            let mut not_visited = not_visited.lock().unwrap();
            let mut visited = visited.lock().unwrap();

            if let Some(url) = not_visited.iter().next() {
                let url = url.to_owned();
                abc = Some(url.clone());

                not_visited.remove(&url);
                visited.insert(url.clone());
            }
        }

        if abc.is_none() {
            if sleep_count >= MAX_SLEEP_COUNT {
                return;
            }

            sleep_count += 1;
            async_std::task::sleep(Duration::from_millis(100)).await;
            continue;
        }

        let task = abc.unwrap();
        sleep_count = 0;

        if let Some(html) = fetch_html_document(&task).await {
            let domain_address = domain_address.lock().unwrap().clone();

            let links = scrap_links(&domain_address, &html).unwrap();

            {
                let mut not_visited = not_visited.lock().unwrap();
                let visited = visited.lock().unwrap();

                for link in links.iter() {
                    if !visited.contains(link) && !not_visited.contains(link) {
                        not_visited.insert(link.to_owned());
                    }
                }
            }
        }
    }
}

pub async fn fetch_html_document(url: &str) -> Option<Html> {
    let mut response = surf::get(url).await.ok()?;

    if response.status() != 200 {
        println!(
            "Unable to fetch url: {}  Status code: {}",
            url,
            response.status()
        );
        return None;
    }

    let document = response.body_string().await.ok()?;
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
    } else if url_source.ends_with(".html") {
        return Some(domain_address.to_owned() + "/" + url_source);
    }

    None
}
