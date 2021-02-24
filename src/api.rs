use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tide::Body;
use tide::{Request, Response};
use url::Url;

use crate::State;

#[derive(Deserialize, Serialize, Debug)]
struct Domain {
    address: String,
    max_depth: Option<usize>,
}

#[derive(Deserialize, Serialize)]
struct PostCrawlAnswer {
    pub id: String,
}

type ThreadShared<T> = Arc<Mutex<T>>;

pub async fn post_crawl(mut req: Request<State>) -> tide::Result {
    let domain: Domain = req.body_json().await?;
    println!("Domain name: {:#?}", domain);

    let key = format!("{:x}", md5::compute(domain.address.clone()));
    let value = test_run(&domain.address).await;
    let value = match value {
        Some(x) => x,
        None => return Ok("error".to_owned().into()),
    };

    let mut links = req.state().links.write().unwrap();
    links.insert(key.clone(), value);

    // Return
    let body = PostCrawlAnswer { id: key };
    let mut res = Response::new(201);
    res.set_body(Body::from_json(&body)?);
    return Ok(res);
}

pub async fn get_crawled_list(req: Request<State>) -> tide::Result {
    let crawls = req.state().links.read().unwrap();
    let id = req.param("id")?;

    if let Some(list) = crawls.get(id) {
        let mut res = Response::new(200);
        res.set_body(Body::from_json(list)?);
        return Ok(res);
    }

    Ok("Failed".into())
}

pub async fn get_crawled_count(req: Request<State>) -> tide::Result {
    let crawls = req.state().links.read().unwrap();
    let id = req.param("id")?;

    if let Some(list) = crawls.get(id) {
        let mut res = Response::new(200);
        res.set_body(Body::from_json(&list.len())?);
        return Ok(res);
    }

    Ok("Failed".into())
}

async fn test_run(domain_address: &str) -> Option<Vec<String>> {
    let mut visited = HashSet::new();
    let mut not_visited: HashSet<String> = HashSet::new();

    visited.insert(domain_address.to_owned());

    let links;
    {
        let html = fetch_html_document(domain_address).await?;
        links = scrap_links(domain_address, &html).unwrap();
    }

    for link in links.iter() {
        if !visited.contains(link) && !not_visited.contains(link) {
            not_visited.insert(link.to_owned());
        }
    }

    let mut visited_state = VisitedState::new();
    visited_state.visited = visited;
    visited_state.not_visited = not_visited;

    let visited_state = Arc::new(Mutex::new(visited_state));
    let active_workers = Arc::new(Mutex::new(0u16));
    let domain_address = Arc::new(Mutex::new(domain_address.to_owned()));

    let mut handles = Vec::new();
    for _i in 0..3 {
        handles.push(async_std::task::spawn(test(
            domain_address.clone(),
            visited_state.clone(),
            active_workers.clone(),
        )));
    }

    for h in handles {
        h.await;
    }
    let visited_state = visited_state.lock().unwrap();
    Some(visited_state.visited.iter().map(|x| x.clone()).collect())
}

pub async fn test(
    domain_address: ThreadShared<String>,
    visited_state: ThreadShared<VisitedState>,
    active_workers: ThreadShared<u16>,
) {
    loop {
        let abc;
        {
            abc = visited_state.lock().unwrap().not_visited_last();
        }

        if abc.is_none() {
            {
                if *active_workers.lock().unwrap() == 0 {
                    return;
                }
            }

            async_std::task::sleep(Duration::from_millis(100)).await;
            continue;
        } else {
            *active_workers.lock().unwrap() += 1;
        }

        let task = abc.unwrap();

        if let Some(html) = fetch_html_document(&task).await {
            let domain_address = domain_address.lock().unwrap().clone();
            let links = scrap_links(&domain_address, &html).unwrap();

            {
                visited_state.lock().unwrap().add_urls(&links);
            }
        }

        *active_workers.lock().unwrap() -= 1;
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

pub struct VisitedState {
    pub visited: HashSet<String>,
    pub not_visited: HashSet<String>,
}

impl VisitedState {
    pub fn new() -> Self {
        Self {
            visited: HashSet::new(),
            not_visited: HashSet::new(),
        }
    }

    pub fn not_visited_last(&mut self) -> Option<String> {
        if let Some(url) = self.not_visited.iter().next() {
            let url = url.clone();
            self.visited.insert(url.clone());
            self.not_visited.remove(&url);
            return Some(url.clone());
        }

        None
    }

    pub fn add_urls(&mut self, links: &[String]) {
        for link in links.iter() {
            if !self.visited.contains(link) && !self.not_visited.contains(link) {
                self.not_visited.insert(link.to_owned());
            }
        }
    }
}
