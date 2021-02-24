use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tide::{Body, Request, Response};
use url::Url;

use crate::State;

#[derive(Deserialize, Serialize, Debug)]
struct Domain {
    address: String,
    max_depth: Option<usize>,
    max_pages: Option<usize>,
}

#[derive(Deserialize, Serialize)]
struct PostCrawlAnswer {
    pub id: String,
}

type ThreadShared<T> = Arc<Mutex<T>>;

pub async fn post_crawl(mut req: Request<State>) -> tide::Result {
    let domain: Domain = req.body_json().await?;
    println!("Domain name: {:#?}", domain);

    let key = format!("{:?}", domain);
    let key = format!("{:x}", md5::compute(key));
    let value = test_run(&domain.address, domain.max_depth, domain.max_pages).await;
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

async fn test_run(
    domain_address: &str,
    max_depth: Option<usize>,
    max_pages: Option<usize>,
) -> Option<Vec<String>> {
    let visited_state = VisitedState::new(domain_address, max_depth, max_pages);

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
        let task;
        {
            task = visited_state.lock().unwrap().not_visited_last();
        }

        if task.is_none() {
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
        let task = task.unwrap();

        let url = task.0.clone();
        let depth = task.1.clone();

        if let Some(document) = fetch_html_document(&url).await {
            let domain_address = domain_address.lock().unwrap().clone();

            let file_name: Vec<&str> = url.split(&domain_address).collect();
            let file_content = document.clone();

            let url = Url::parse(&domain_address).unwrap();
            let domain_name = url.domain().unwrap();

            let mut a: String = file_name[1].to_owned();
            if a.starts_with('/') {
                a = a[1..].to_owned();
            }

            save_to_file(domain_name, &a, &file_content).await;
            let html = Html::parse_document(document.as_str());
            let links = scrap_links(&domain_address, &html).unwrap();

            {
                visited_state.lock().unwrap().add_urls(&links, depth);
            }
        }

        *active_workers.lock().unwrap() -= 1;
    }
}

pub async fn fetch_html_document(url: &str) -> Option<String> {
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
    Some(document)
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
    visited: HashSet<String>,
    not_visited: HashMap<String, usize>,
    max_depth: Option<usize>,
    max_pages: Option<usize>,
}

impl VisitedState {
    pub fn new(base_url: &str, max_depth: Option<usize>, max_pages: Option<usize>) -> Self {
        let mut not_visited = HashMap::new();
        not_visited.insert(base_url.to_owned(), 0);

        Self {
            visited: HashSet::new(),
            not_visited,
            max_depth,
            max_pages,
        }
    }

    pub fn not_visited_last(&mut self) -> Option<(String, usize)> {
        if let Some(max_pages) = self.max_pages {
            if self.visited.len() >= max_pages {
                return None;
            }
        }

        if let Some(visitation) = self.not_visited.iter().next() {
            let url = visitation.0.clone();
            let depth = visitation.1.clone();

            self.visited.insert(url.clone());
            self.not_visited.remove(&url);

            return Some((url.clone(), depth));
        }

        None
    }

    pub fn add_urls(&mut self, links: &[String], depth: usize) {
        let new_depth = depth + 1;

        if let Some(max_depth) = self.max_depth {
            if new_depth > max_depth {
                return;
            }
        }

        for link in links.iter() {
            if self.visited.contains(link) {
                continue;
            }

            if let Some(entry_depth) = self.not_visited.get_mut(link) {
                *entry_depth = (*entry_depth).min(new_depth);
            } else {
                self.not_visited.insert(link.to_owned(), new_depth);
            }
        }
    }
}

pub async fn save_to_file(domain_name: &str, path: &str, contents: &str) -> Option<bool> {
    async_std::fs::create_dir_all(format!("downloaded/{}/{}", domain_name, path))
        .await
        .ok()?;
    async_std::fs::write(
        format!("downloaded/{}/{}/index.html", domain_name, path),
        contents,
    )
    .await
    .ok()?;

    Some(true)
}
