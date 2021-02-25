use super::state::{SpiderState, SpiderTask};
use crate::common::ThreadShared;
use crate::state::Database;
use async_std::sync::{Arc, Mutex};
use http_types::Url;
use scraper::{Html, Selector};
use std::time::Duration;

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
    ) -> Vec<String> {
        // Check if the base_url is valid
        if !is_url_valid(&base_url).await {
            return vec![];
        }

        // Create a task-shared state
        let state = SpiderState::new(base_url, options);
        let state = Arc::new(Mutex::new(state));

        let mut tasks = Vec::new();

        // TODO: Make the number of running tasks to be settable
        // by a Input parameter
        for _i in 0..50 {
            tasks.push(async_std::task::spawn(run_task(
                state.clone(),
                database.clone(),
            )));
        }

        // Wait for all the tasks finish
        for h in tasks {
            h.await;
        }

        // Retrieve the list of visited pages
        let state = state.lock().await;
        state.visited.iter().map(|x| x.clone()).collect()
    }
}

pub async fn is_url_valid(url: &str) -> bool {
    fetch_document(url).await.is_some()
}

pub async fn run_task(state: ThreadShared<SpiderState>, database: ThreadShared<Database>) {
    // Retrieve unchangeable data
    // This also could have been done by passing those objects
    // as parameters of this function
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
            // The task is not done when there is still something to do
            // or if there are still workers (other tasks) running
            let mut state = state.lock().await;
            task = state.task();
            if task.is_some() {
                state.active_workers += 1;
            } else if state.active_workers == 0 {
                return;
            }
        }

        // If there is nothing to do then sleep for a while
        if task.is_none() {
            async_std::task::sleep(Duration::from_millis(100)).await;
            continue;
        }
        let SpiderTask { url, depth } = task.unwrap();

        let mut document;
        {
            // Get the html document from our database
            let database = database.lock().await;
            document = database.crawled_urls.get(&url).map(|x| x.clone());

            // If not found, try to fetch it
            if document.is_none() {
                document = fetch_document(&url).await;
            }
        }

        if let Some(document) = document {
            // Save the document so that next time when we need it we can
            // can read it from our database
            //
            // For sake of simplicity there is no additional checks if the key
            // already exists in the map
            {
                let mut database = database.lock().await;
                database.crawled_urls.insert(url.clone(), document.clone());
            }

            // Archived (downloaded) pages will be stored in the "download" folder
            if archive_pages {
                save_to_file(&base_url, &url, &document).await;
            }

            let links;
            {
                // Scrap all the links found inside HTML object
                let html = Html::parse_document(document.as_str());
                links = scrap_links(&base_url, &html);
            }

            {
                // Add the found links to the list of not visited links
                state.lock().await.add_links(&links, depth);
            }
        }

        {
            state.lock().await.active_workers -= 1;
        }
    }
}

pub async fn fetch_document(url: &str) -> Option<String> {
    let mut response = surf::get(url).await.ok()?;

    if response.status() != 200 {
        let code = response.status();
        println!("Unable to fetch url: {}  Status code: {}", url, code);
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
    let url = Url::parse(url).ok()?;
    let subpath = match url.path() {
        "/" => String::new(),
        x => x.to_owned(),
    };

    let url = Url::parse(base_url).ok()?;
    let domain = url.domain()?;

    let folder_path = format!("downloaded/{}/{}", domain, subpath);
    async_std::fs::create_dir_all(folder_path).await.ok()?;

    let future = if subpath.is_empty() {
        let file_path = format!("downloaded/{}/index.html", domain);
        async_std::fs::write(file_path, contents)
    } else {
        let file_path = format!("downloaded/{}/{}/index.html", domain, subpath);
        async_std::fs::write(file_path, contents)
    };
    future.await.ok()?;

    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_document_ok() {
        const SOURCE: &str = r#"<style type="text/css">
        h1 {
            text-align: center;
            font-size: 120px;
            font-family: Helvetica, Verdana, Arial;
        }
        </style>
        <h1>You spelled it wrong.</h1>"#;

        let result = async_std::task::block_on(fetch_document("https://guthib.com/"));
        assert!(result.is_some());

        let actual = result.unwrap().replace(" ", "");
        let actual = actual.replace("\n", "");
        let expected = SOURCE.to_owned().replace(" ", "");
        let expected = expected.replace("\n", "");

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_fetch_html_document_bad() {
        let result = async_std::task::block_on(fetch_document("https://guthibb.com/"));
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
        <h1>You spelled it wrong.</h1>"#;

        let base_url = "https://www.test.com";
        let html = Html::parse_document(SOURCE);

        let links = scrap_links(base_url, &html);
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn spider_run_case_1_ok() {
        let base_url = "http://www.zadruga-podolski.hr".to_owned();
        let options = SpiderOptions::new(None, None, false, false);
        let database = Arc::new(Mutex::new(Database::new()));

        let links = async_std::task::block_on(Spider::run(base_url, options, database));
        assert_eq!(links.len(), 13);
    }

    #[test]
    fn spider_run_case_max_depth_ok() {
        let base_url = "http://www.zadruga-podolski.hr".to_owned();
        let options = SpiderOptions::new(Some(0), None, false, false);
        let database = Arc::new(Mutex::new(Database::new()));

        let links = async_std::task::block_on(Spider::run(base_url, options, database));
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn spider_run_case_max_pages_ok() {
        const MAX_PAGES: usize = 3;

        let base_url = "http://www.zadruga-podolski.hr".to_owned();
        let options = SpiderOptions::new(None, Some(MAX_PAGES), false, false);
        let database = Arc::new(Mutex::new(Database::new()));

        let links = async_std::task::block_on(Spider::run(base_url, options, database));
        assert_eq!(links.len(), MAX_PAGES);
    }

    #[test]
    fn spider_run_case_unknown_domain_ok() {
        let base_url = "http://www.zadruga-tafatefe2.hr".to_owned();
        let options = SpiderOptions::new(None, None, false, false);
        let database = Arc::new(Mutex::new(Database::new()));

        let links = async_std::task::block_on(Spider::run(base_url, options, database));
        assert!(links.is_empty());
    }
}
