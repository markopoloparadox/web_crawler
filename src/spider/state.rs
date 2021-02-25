use std::collections::{HashMap, HashSet};

use super::SpiderOptions;

pub struct SpiderState {
    pub base_url: String,
    pub visited: HashSet<String>,
    pub not_visited: HashMap<String, usize>,
    pub options: SpiderOptions,
    pub active_workers: u16,
}

impl SpiderState {
    pub fn new(base_url: String, options: SpiderOptions) -> Self {
        let mut not_visited = HashMap::new();
        not_visited.insert(base_url.to_owned(), 0);

        Self {
            base_url,
            visited: HashSet::new(),
            not_visited,
            options,
            active_workers: 0,
        }
    }

    pub fn task(&mut self) -> Option<SpiderTask> {
        if let Some(max_pages) = self.options.max_pages {
            if self.visited.len() >= max_pages {
                return None;
            }
        }

        if let Some(visitation) = self.not_visited.iter().next() {
            let url = visitation.0.clone();
            let depth = visitation.1.clone();

            self.visited.insert(url.clone());
            self.not_visited.remove(&url);

            return Some(SpiderTask { url, depth });
        }

        None
    }

    pub fn add_urls(&mut self, links: &[String], depth: usize) {
        let new_depth = depth + 1;

        if let Some(max_depth) = self.options.max_depth {
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

pub struct SpiderTask {
    pub url: String,
    pub depth: usize,
}
