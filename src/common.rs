use async_std::sync::{Arc, Mutex};

pub type ThreadShared<T> = Arc<Mutex<T>>;
