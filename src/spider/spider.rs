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
        base_url: String,
        options: SpiderOptions,
        database: ThreadShared<Database>,
    ) -> Option<Vec<String>> {
        let state = SpiderState::new(base_url, options);
        let state = Arc::new(Mutex::new(state));

        let mut handles = Vec::new();
        for _i in 0..10 {
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
    let base_url;
    let archive_pages;
    {
        let state = state.lock().await;
        base_url = state.base_url.clone();
        archive_pages = state.options.archive_pages;
    }

    loop {
        let task;
        {
            let mut state = state.lock().await;
            task = state.task();
            if task.is_some() {
                state.active_workers += 1;
            } else if state.active_workers == 0 {
                return;
            }
        }

        if task.is_none() {
            async_std::task::sleep(Duration::from_millis(100)).await;
            continue;
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
                save_to_file(&base_url, &url, &document).await;
            }

            let links;
            {
                let html = Html::parse_document(document.as_str());
                links = scrap_links(&base_url, &html);
            }

            {
                state.lock().await.add_urls(&links, depth);
            }
        }

        {
            state.lock().await.active_workers -= 1;
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

pub fn scrap_links(base_url: &str, html: &Html) -> Vec<String> {
    let selector = match Selector::parse("a[href]") {
        Ok(x) => x,
        Err(_) => return vec![],
    };

    let elements = html.select(&selector);
    let links: Vec<String> = elements
        .filter_map(|x| x.value().attr("href"))
        .filter_map(|x| normalize_url(base_url, x))
        .collect();

    links
}

pub fn normalize_url(base_url: &str, url: &str) -> Option<String> {
    if url.starts_with(base_url) {
        return Some(url.to_owned());
    } else if url.starts_with('/') {
        return Some(base_url.to_owned() + url);
    } else if url.ends_with(".html") {
        return Some(base_url.to_owned() + "/" + url);
    }

    None
}

pub async fn save_to_file(base_url: &str, url: &str, contents: &str) -> Option<bool> {
    let file_name: Vec<&str> = url.split(&base_url).collect();
    let mut path: String = file_name[1].to_owned();
    if path.starts_with('/') {
        path = path[1..].to_owned();
    }

    let url = Url::parse(&base_url).unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_html_document_ok() {
        const SOURCE: &str = r#"<style type="text/css">
        h1 {
            text-align: center;
            font-size: 120px;
            font-family: Helvetica, Verdana, Arial;
        }
        </style>
        <h1>You spelled it wrong.</h1>"#;

        let result = async_std::task::block_on(fetch_html_document("https://guthib.com/"));
        assert!(result.is_some());

        let actual = result.unwrap().replace(" ", "");
        let actual = actual.replace("\n", "");
        let expected = SOURCE.to_owned().replace(" ", "");
        let expected = expected.replace("\n", "");

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_fetch_html_document_bad() {
        let result = async_std::task::block_on(fetch_html_document("https://guthibb.com/"));
        assert!(result.is_none());
    }

    #[test]
    fn test_normalize_url_case_full_path_ok() {
        let base_url = "https://www.test.com";
        let url = "https://www.test.com";

        let result = normalize_url(base_url, url);
        assert!(result.is_some());

        let actual = result.unwrap();
        let expected = url.to_owned();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_normalize_url_case_html_path_ok() {
        let base_url = "https://www.test.com";
        let url = "index.html";

        let result = normalize_url(base_url, url);
        assert!(result.is_some());

        let actual = result.unwrap();
        let expected = base_url.to_owned() + "/" + url;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_normalize_url_case_part_path_ok() {
        let base_url = "https://www.test.com";
        let url = "/route";

        let result = normalize_url(base_url, url);
        assert!(result.is_some());

        let actual = result.unwrap();
        let expected = base_url.to_owned() + url;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_normalize_url_case_1_bad() {
        let base_url = "https://www.test.com";
        let url = "route";

        let result = normalize_url(base_url, url);
        assert!(result.is_none());
    }

    #[test]
    fn test_scrap_links_case_two_links_ok() {
        const SOURCE: &str = r#"
        <a href="/Test1"></a>
        <a href="/Test2"></a>"#;

        let base_url = "https://www.test.com";
        let html = Html::parse_document(SOURCE);

        let links = scrap_links(base_url, &html);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0], base_url.to_owned() + "/Test1");
        assert_eq!(links[1], base_url.to_owned() + "/Test2");
    }

    #[test]
    fn test_scrap_links_case_zero_links_ok() {
        const SOURCE: &str = r#"
        <a></a>
        <h1>You spelled it wrong.</h1>""#;

        let base_url = "https://www.test.com";
        let html = Html::parse_document(SOURCE);

        let links = scrap_links(base_url, &html);
        assert_eq!(links.len(), 0);
    }
}
