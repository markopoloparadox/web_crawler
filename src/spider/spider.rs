use super::state::{SpiderState, SpiderTask};
use crate::state::Database;
use async_std::sync::{Arc, Mutex};
use http_types::Url;
use scraper::{Html, Selector};
use std::time::Duration;

pub type ThreadShared<T> = Arc<Mutex<T>>;

pub struct SpiderOptions {
    pub max_depth: Option<usize>,
    pub max_pages: Option<usize>,
    pub robots_txt: bool,
    pub archive_pages: bool,
}

impl SpiderOptions {
    pub fn new(
        max_depth: Option<usize>,
        max_pages: Option<usize>,
        robots_txt: bool,
        archive_pages: bool,
    ) -> Self {
        Self {
            max_depth,
            max_pages,
            robots_txt,
            archive_pages,
        }
    }
}

pub struct Spider;
impl Spider {
    pub async fn run(
        domain_address: &str,
        options: SpiderOptions,
        database: ThreadShared<Database>,
    ) -> Option<Vec<String>> {
        let state = SpiderState::new(domain_address, options);
        let state = Arc::new(Mutex::new(state));

        let mut handles = Vec::new();
        for _i in 0..3 {
            handles.push(async_std::task::spawn(task(
                state.clone(),
                database.clone(),
            )));
        }

        for h in handles {
            h.await;
        }
        let state = state.lock().await;
        Some(state.visited.iter().map(|x| x.clone()).collect())
    }
}

pub async fn task(state: ThreadShared<SpiderState>, database: ThreadShared<Database>) {
    let domain_address;
    let archive_pages;
    {
        let state = state.lock().await;
        domain_address = state.domain_address.clone();
        archive_pages = state.options.archive_pages;
    }

    loop {
        let task;
        let active_workers;
        {
            let mut state = state.lock().await;
            task = state.task();
            active_workers = state.active_workers.clone();
        }

        if task.is_none() {
            if active_workers == 0 {
                return;
            }

            async_std::task::sleep(Duration::from_millis(100)).await;
            continue;
        } else {
            state.lock().await.active_workers += 1;
        }

        let SpiderTask { url, depth } = task.unwrap();

        // Check if the document is already retrieved for requested url
        let mut document;
        {
            let database = database.lock().await;
            document = database.crawled_urls.get(&url).map(|x| x.clone());
        }

        // If is not inside the database, try to fetch it
        if document.is_none() {
            document = fetch_html_document(&url).await;
        }

        if let Some(document) = document {
            {
                let mut database = database.lock().await;
                database.crawled_urls.insert(url.clone(), document.clone());
            }

            if archive_pages {
                save_to_file(&domain_address, &url, &document).await;
            }

            let links;
            {
                let html = Html::parse_document(document.as_str());
                links = scrap_links(&domain_address, &html).unwrap();
            }

            {
                state.lock().await.add_urls(&links, depth);
            }
        }

        {
            state.lock().await.active_workers += 1;
        }
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

pub async fn save_to_file(domain_address: &str, url: &str, contents: &str) -> Option<bool> {
    let file_name: Vec<&str> = url.split(&domain_address).collect();
    let mut path: String = file_name[1].to_owned();
    if path.starts_with('/') {
        path = path[1..].to_owned();
    }

    let url = Url::parse(&domain_address).unwrap();
    let domain_name = url.domain().unwrap();

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
