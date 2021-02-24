use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct State {
    pub links: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            links: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
