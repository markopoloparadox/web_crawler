use std::collections::HashMap;

use async_std::sync::{Arc, Mutex};

use crate::spider::ThreadShared;

#[derive(Clone)]
pub struct State {
    pub database: ThreadShared<Database>,
}

impl State {
    pub fn new() -> Self {
        Self {
            database: Arc::new(Mutex::new(Database::new())),
        }
    }
}

pub struct Database {
    // HashMap<Spider_id, Vec<urls>>
    pub domain_links: HashMap<String, Vec<String>>,

    // HashMap<url, data>
    pub crawled_urls: HashMap<String, String>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            crawled_urls: HashMap::new(),
            domain_links: HashMap::new(),
        }
    }
}
